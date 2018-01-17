# rust-varlink
WIP

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