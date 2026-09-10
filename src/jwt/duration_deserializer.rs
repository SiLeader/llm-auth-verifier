use chrono::Duration;

#[derive(Debug, Clone, Copy)]
pub(super) struct Dur(pub Duration);

struct DurVisitor;

impl<'de> serde::de::Visitor<'de> for DurVisitor {
    type Value = Dur;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string representing a duration, e.g., '1h', '30m', '15s'")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let dur = humantime::parse_duration(value)
            .map_err(|e| E::custom(format!("failed to parse duration '{}': {}", value, e)))?;
        Ok(Dur(Duration::from_std(dur).map_err(|e| {
            E::custom(format!(
                "failed to convert std::time::Duration to chrono::Duration: {}",
                e
            ))
        })?))
    }
}

impl<'de> serde::Deserialize<'de> for Dur {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DurVisitor)
    }
}
