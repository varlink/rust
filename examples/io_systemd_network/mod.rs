use serde_json;

use std::result::Result;
use std::convert::From;
use std::borrow::Cow;

use varlink::server::Interface as VarlinkInterface;
use varlink::server::Error as VarlinkError;

pub trait Interface: VarlinkInterface {
    fn info(&self, i64) -> Result<InfoReply, Error>;
    fn list(&self) -> Result<ListReply, Error>;
}

#[macro_export]
macro_rules! IoSystemdNetwork {
	(
		()
		$(pub)* struct $name:ident $($_tail:tt)*
	) => {
use varlink::server::{Request};
use varlink::server::Interface as VarlinkInterface;
use varlink::server::Error as VarlinkError;
use serde_json::Value as SerdeValue;

impl VarlinkInterface for $name {
    fn get_description(&self) -> &'static str {
        r#"
# Provides information about network state
interface io.systemd.network

type NetdevInfo (
  ifindex: int,
  ifname: string
)

type Netdev (
  ifindex: int,
  ifname: string
)

# Returns information about a network device
method Info(ifindex: int) -> (info: NetdevInfo)

# Lists all network devices
method List() -> (netdevs: Netdev[])

error UnknownNetworkDevice ()
error InvalidParameter (field: string)
	"#
    }

    fn get_name(&self) -> &'static str {
        "io.systemd.network"
    }

    fn call(&self, req: Request) -> Result<SerdeValue, VarlinkError> {
        match req.method.as_ref() {
            "io.systemd.network.Info" => {
                if let Some(args) = req.parameters {
                    let infoargs: Result<InfoArgs, serde_json::Error> =
                        serde_json::from_value(args);
                    match infoargs {
                        Ok(args) => Ok(serde_json::to_value(self.info(args.ifindex)?)?),
                        Err(_) => Err(Error::InvalidParameter.into()),
                    }
                } else {
                    Err(Error::InvalidParameter.into())
                }
            }
            "io.systemd.network.List" => Ok(serde_json::to_value(self.list()?)?),
            m => {
                let method: String = m.clone().into();
                Err(Error::MethodNotFound(Some(method.into())).into())
            }
        }
    }
}
};
}

#[derive(Debug)]
pub enum Error {
    UnknownNetworkDevice,
    InvalidParameter,
    MethodNotFound(Option<Cow<'static, str>>),
    UnknownError(Option<Cow<'static, str>>),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::UnknownError(Some(e.to_string().into()))
    }
}

impl From<Error> for VarlinkError {
    fn from(e: Error) -> Self {
        VarlinkError {
            error: match e {
                Error::UnknownNetworkDevice => "io.systemd.network.UnknownNetworkDevice".into(),
                Error::InvalidParameter => "org.varlink.service.InvalidParameter".into(),
                Error::MethodNotFound(_) => "org.varlink.service.MethodNotFound".into(),
                _ => "UnknownError".into(),
            },
            parameters: match e {
                Error::MethodNotFound(m) => {
                    match m {
                        Some(me) => {
                            let method: String = me.into();
                            let n: usize = match method.rfind('.') {
                                None => 0,
                                Some(x) => x + 1,
                            };
                            let (_, method) = method.split_at(n);
                            let s = format!("{{  \"method\" : \"{}\" }}", method);
                            Some(serde_json::from_str(s.as_ref()).unwrap())
                        }
                        None => None,
                    }
                }
                _ => None,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetdevInfo {
    pub ifindex: i64,
    pub ifname: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Netdev {
    pub ifindex: i64,
    pub ifname: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoArgs {
    pub ifindex: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoReply {
    pub info: NetdevInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListReply {
    pub netdevs: Vec<Netdev>,
}
