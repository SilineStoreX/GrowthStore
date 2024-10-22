use fastdate::time1::format_description;
use rhai::CustomType;
use rhai::TypeBuilder;

#[derive(Clone, CustomType)]
pub struct RhaiDateTime {
    inner: fastdate::DateTime,
}

impl RhaiDateTime {
    pub fn now() -> Self {
        Self {
            inner: fastdate::DateTime::now(),
        }
    }

    pub fn now_utc() -> Self {
        Self {
            inner: fastdate::DateTime::utc(),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&mut self) -> String {
        self.inner.format("YYYY-MM-DD hh:mm:ss")
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_global(&mut self) -> String {
        self.inner.inner.to_string()
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_rfc2822(&mut self) -> String {
        let fmt = format_description::well_known::Rfc2822;
        self.inner.inner.format(&fmt).unwrap_or_default()
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_locale(&mut self) -> String {
        let fmt = format_description::well_known::Rfc2822;
        self.inner.inner.format(&fmt).unwrap_or_default()
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_locale_fmt(&mut self, fmt: &str) -> String {
        match format_description::parse(fmt) {
            Ok(fmtt) => self.inner.inner.format(&fmtt).unwrap_or_default(),
            Err(_err) => self.to_locale(),
        }
    }

    pub fn format(&mut self, fmt: &str) -> String {
        self.inner.format(fmt)
    }

    pub fn parse(&mut self, val: &str, fmt: &str) -> RhaiDateTime {
        match fastdate::DateTime::parse(fmt, val) {
            Ok(t) => Self { inner: t },
            Err(_) => Self::now(),
        }
    }

    pub fn parse_default(&mut self, val: &str) -> RhaiDateTime {
        match fastdate::DateTime::parse("YYYY-MM-DD hh:mm:ss", val) {
            Ok(t) => Self { inner: t },
            Err(_) => Self::now(),
        }
    }

    pub fn year(&mut self) -> i32 {
        self.inner.year()
    }

    pub fn month(&mut self) -> i32 {
        self.inner.mon() as i32
    }

    pub fn day(&mut self) -> i32 {
        self.inner.day() as i32
    }

    pub fn hour(&mut self) -> i32 {
        self.inner.hour() as i32
    }

    pub fn minute(&mut self) -> i32 {
        self.inner.minute() as i32
    }

    pub fn second(&mut self) -> i32 {
        self.inner.sec() as i32
    }

    pub fn timestamp_millis(&mut self) -> i64 {
        self.inner.unix_timestamp_millis()
    }

    pub fn timestamp_micro(&mut self) -> i64 {
        self.inner.unix_timestamp_micros()
    }

    pub fn timestamp_second(&mut self) -> i64 {
        self.inner.unix_timestamp_millis() / 1000
    }
}
