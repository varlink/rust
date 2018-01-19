# rust-varlink
WIP

## varlink tool installaiton

```bash
$ sudo dnf copr enable "@varlink/varlink"
$ sudo dnf install fedora-varlink
$ sudo setenforce 0 # needed until systemd is able to create sockets in /run
$ sudo systemctl enable --now org.varlink.resolver.socket
$ varlink help
```

## varlink file validator
```
$ cargo run --example validate examples/io_systemd_network/io.systemd.network.varlink 
    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs
     Running `target/debug/examples/validate examples/io_systemd_network/io.systemd.network.varlink`
Syntax check passed!

interface io.systemd.network
type Netdev (ifindex: int, ifname: string)
type NetdevInfo (ifindex: int, ifname: string)
method Info(ifindex: int) -> (info: NetdevInfo)
method List() -> (netdevs: Netdev[])
error InvalidParameter (field: string)
error UnknownNetworkDevice ()
```

## varlink rust generator
```
$ cargo run --example varlink-generator examples/io_systemd_network/io.systemd.network.varlink 
    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs
     Running `target/debug/examples/varlink-generator examples/io_systemd_network/io.systemd.network.varlink`
#[derive(Serialize, Deserialize, Debug)]
pub struct Netdev {
    pub ifindex : i64,
    pub ifname : String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetdevInfo {
    pub ifindex : i64,
    pub ifname : String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoReply {
    pub info : NetdevInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoArgs {
    pub ifindex : i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListReply {
    pub netdevs : Vec<Netdev>,
}
```

## Example Server

```
$ cargo run --example server 
```

and test from a new shell

```
$ varlink help ip:127.0.0.1:12345/org.varlink.service
# The Varlink Service Interface is provided by every varlink service. It
# describes the service and the interfaces it implements.
interface org.varlink.service

# Get a list of all the interfaces a service provides and information
# about the implementation.
method GetInfo() -> (
  vendor: string,
  product: string,
  version: string,
  url: string,
  interfaces: string[]
)

# Get the description of an interface that is implemented by this service.
method GetInterfaceDescription(interface: string) -> (description: string)

# The requested interface was not found.
error InterfaceNotFound (interface: string)

# The requested method was not found
error MethodNotFound (method: string)

# The interface defines the requested method, but the service does not
# implement it.
error MethodNotImplemented (method: string)

# One of the passed parameters is invalid.
error InvalidParameter (parameter: string)
```

```
$ varlink help ip:127.0.0.1:12345/io.systemd.network
# Provides information about network state
interface io.systemd.network

type NetdevInfo (ifindex: int, ifname: string)

type Netdev (ifindex: int, ifname: string)

# Returns information about a network device
method Info(ifindex: int) -> (info: NetdevInfo)

# Lists all network devices
method List() -> (netdevs: Netdev[])

error UnknownNetworkDevice ()

error InvalidParameter (field: string)
```

```
$ varlink call ip:127.0.0.1:12345/io.systemd.network.List
{
  "netdevs": [
    {
      "ifindex": 1,
      "ifname": "lo"
    },
    {
      "ifindex": 2,
      "ifname": "eth0"
    }
  ]
}
```
