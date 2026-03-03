use anidb_shared_types::events::EventMetadata;
use anidb_shared_types::Event;
use eventstore::{AppendToStreamOptions, Client, ClientSettings, EventData, ReadStreamOptions};
use uuid::Uuid;

use crate::error::EventLogError;

pub struct EventStoreClient {
    inner: Client,
}

impl EventStoreClient {
    pub fn new(connection_string: &str) -> Result<Self, EventLogError> {
        let settings: ClientSettings = connection_string
            .parse()
            .map_err(|e: eventstore::ClientSettingsParseError| {
                EventLogError::Store(e.to_string())
            })?;
        let client =
            Client::new(settings).map_err(|e| EventLogError::Store(e.to_string()))?;
        Ok(Self { inner: client })
    }

    pub async fn append(
        &self,
        stream_name: &str,
        event_type: &str,
        payload: &serde_json::Value,
        metadata: &EventMetadata,
    ) -> Result<Uuid, EventLogError> {
        let event_id = Uuid::new_v4();
        let evt = EventData::json(event_type, payload)
            .map_err(|e| EventLogError::Store(e.to_string()))?
            .id(event_id)
            .metadata_as_json(metadata)
            .map_err(|e| EventLogError::Store(e.to_string()))?;
        self.inner
            .append_to_stream(stream_name, &AppendToStreamOptions::default(), evt)
            .await
            .map_err(|e| EventLogError::Store(e.to_string()))?;
        Ok(event_id)
    }

    pub async fn read_stream(&self, stream_name: &str) -> Result<Vec<Event>, EventLogError> {
        let options = ReadStreamOptions::default();
        let mut stream = self
            .inner
            .read_stream(stream_name, &options)
            .await
            .map_err(|e: eventstore::Error| match e {
                eventstore::Error::ResourceNotFound => {
                    EventLogError::StreamNotFound(stream_name.to_string())
                }
                other => EventLogError::Store(other.to_string()),
            })?;

        let mut events = Vec::new();
        while let Some(resolved) = stream.next().await.map_err(|e: eventstore::Error| match e {
            eventstore::Error::ResourceNotFound => {
                EventLogError::StreamNotFound(stream_name.to_string())
            }
            other => EventLogError::Store(other.to_string()),
        })? {
            let original = resolved.get_original_event();
            let payload: serde_json::Value = original
                .as_json()
                .map_err(|e| EventLogError::Store(e.to_string()))?;
            let metadata: EventMetadata =
                serde_json::from_slice(original.custom_metadata.as_ref())?;
            events.push(Event {
                id: original.id,
                stream_id: original.stream_id.to_string(),
                event_type: original.event_type.to_string(),
                payload,
                metadata,
            });
        }
        Ok(events)
    }

    pub async fn read_by_category(&self, category: &str) -> Result<Vec<Event>, EventLogError> {
        let system_stream = format!("$ce-{}", category);
        self.read_stream(&system_stream).await
    }
}
