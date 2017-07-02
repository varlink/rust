use std::result::Result;
use std::convert::From;
use std::borrow::Cow;

use serde_json;

use varlink::server::ErrorDetails;
use varlink::server::Interface as VarlinkInterface;
use varlink::server::Error as VarlinkError;

pub trait Interface: VarlinkInterface {
    fn info(&self, i64) -> Result<InfoRet, Error>;
    fn list(&self) -> Result<ListRet, Error>;
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

error UnknownNetworkDevice
error InvalidParameter
	"#
    }

    fn get_name(&self) -> &'static str {
        "io.systemd.network"
    }

    fn call(&self, req: Request) -> Result<SerdeValue, VarlinkError> {
        match req.method.as_ref() {
            "Info" => {
                if let Some(args) = req.arguments {
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
            "List" => Ok(serde_json::to_value(self.list()?)?),
            _ => Err(Error::MethodNotFound.into()),
        }
    }
}
};
}

#[derive(Debug)]
pub enum Error {
    UnknownNetworkDevice,
    InvalidParameter,
    MethodNotFound,
    UnknownError(Option<Cow<'static, str>>),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::UnknownError(Some(e.to_string().into()))
    }
}

impl From<Error> for VarlinkError {
    fn from(e: Error) -> Self {
        match e {
            Error::UnknownNetworkDevice => {
                VarlinkError {
                    error: ErrorDetails {
                        id: "UnknownNetworkDevice".into(),
                        ..Default::default()
                    },
                }
            }
            Error::InvalidParameter => {
                VarlinkError {
                    error: ErrorDetails {
                        id: "InvalidParameter".into(),
                        ..Default::default()
                    },
                }
            }
            _ => {
                VarlinkError {
                    error: ErrorDetails {
                        id: "UnknownError".into(),
                        ..Default::default()
                    },
                }
            }
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
pub struct InfoRet {
    pub info: NetdevInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListRet {
    pub netdevs: Option<Vec<Netdev>>,
}
