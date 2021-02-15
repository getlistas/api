use chrono::ParseError;
use chrono::NaiveDate;
use wither::bson::DateTime;

pub fn to_rfc3339(date: DateTime) -> String {
  date.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn from_rfc3339(date: &str) -> Result<DateTime, ParseError> {
  let date: DateTime = chrono::DateTime::parse_from_rfc3339(date)?
    .with_timezone(&chrono::Utc)
    .into();

  Ok(date)
}

// Year-month-day format (ISO 8601). Example: 2001-07-08.
pub fn from_ymd(ymd: &str) -> Result<DateTime, ParseError> {
  let date = format!("{} 00:00:00 +0000", ymd);
  let format = "%Y-%m-%d %H:%M:%S %z";
  let date: DateTime = chrono::DateTime::parse_from_str(date.as_str(), format)?
    .with_timezone(&chrono::Utc)
    .into();

  Ok(date)
}

pub fn now() -> DateTime {
  chrono::Utc::now().into()
}
