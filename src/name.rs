use std::str::FromStr;
use std::num::ParseIntError;

use abstract_ns::name::{self, Name};
use quick_error::ResultExt;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Name(name: String, err: name::Error) {
            cause(err)
            context(name: &'a str, err: name::Error)
                -> (name.to_string(), err)
        }
        Port(name: String, err: ParseIntError) {
            cause(err)
            context(name: &'a str, err: ParseIntError)
                -> (name.to_string(), err)
        }
    }
}

/// A name type that can be read from config
///
/// The core idea of `AutoName` is that for service with default port `80`
/// (HTTP) we treat names the following way:
///
/// * `example.org` → A record example.org, port 80
/// * `example.org:8080` → A record example.org, port 8080
/// * `_service._proto.example.org` → SRV record, and port from the record
///
/// This works by wrapping the string read from configuration file into
/// `AutoName::Auto` and using it in `Router`. You might override things
/// via configuration specific things, for example in yaml you might want
/// to write:
///
/// ```yaml
/// addresses:
/// - !Srv myservice.query.consul
/// ```
///
/// ... And convert it into `Service("myservice.query.consul")` which will
/// be resolved using ``SRV`` record (or similar mechanism) instead of
/// using hostname (i.e. standard expects using `_service._proto` prefix but
/// does not requires that).
#[derive(Debug)]
pub enum AutoName<'a> {
    /// Auto-determine how to treat the name
    Auto(&'a str),
    /// Resolve host and attach specified port
    HostPort(&'a str, u16),
    /// Resolve host and attach default port to it
    HostDefaultPort(&'a str),
    /// Use service name and port resolved using SRV record or similar
    Service(&'a str),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InternalName {
    HostPort(Name, u16),
    Service(Name),
}

impl<'a> AutoName<'a> {
    pub(crate) fn parse(&self, default_port: u16)
        -> Result<InternalName, Error>
    {
        use self::AutoName as A;
        use self::InternalName as I;
        match *self {
            A::Auto(x) => {
                if x.starts_with("_") {
                    Ok(I::Service(Name::from_str(x).context(x)?))
                } else if let Some(pos) = x.find(':') {
                    Ok(I::HostPort(Name::from_str(&x[..pos]).context(x)?,
                                   x[pos+1..].parse().context(x)?))
                } else {
                    Ok(I::HostPort(Name::from_str(x).context(x)?,
                                   default_port))
                }
            }
            A::HostPort(name, port)
            => Ok(I::HostPort(Name::from_str(name).context(name)?, port)),
            A::HostDefaultPort(name)
            => Ok(I::HostPort(Name::from_str(name).context(name)?, default_port)),
            A::Service(name)
            => Ok(I::Service(Name::from_str(name).context(name)?)),
        }
    }
}

impl<'a, T: AsRef<str> + 'a> From<&'a T> for AutoName<'a> {
    fn from(val: &'a T) -> AutoName<'a> {
        AutoName::Auto(val.as_ref())
    }
}

#[cfg(test)]
mod test {
    use abstract_ns::Name;
    use super::AutoName as A;
    use super::InternalName as I;

    fn name(name: &str) -> Name {
        name.parse().unwrap()
    }

    #[test]
    fn auto() {
        assert_eq!(A::Auto("localhost").parse(1234).unwrap(),
            I::HostPort(name("localhost"), 1234));
        assert_eq!(A::Auto("localhost:8080").parse(1234).unwrap(),
            I::HostPort(name("localhost"), 8080));
        assert_eq!(A::Auto("_my._svc.localhost").parse(1234).unwrap(),
            I::Service(name("_my._svc.localhost")));
    }

    #[test]
    #[should_panic(expected="InvalidChar")]
    fn bad_names() {
        A::Auto("_my._svc.localhost:8080").parse(1234).unwrap();
    }
}

