# zinc-observe

🚀 10x easier, 🚀 10x cheaper, 🚀 petabyte scale - Elasticsearch alternative for 🚀 (logs, metrics, traces).

[zinc.dev](https://zinc.dev/)


## Quick Start

```
ZO_USER_NAME="admin" ZO_USER_PASSWORD="Complexpass#123" cargo run
```

## Etcd client

etcd-client which is used by zinc-observe needs protocol buffer compiler, please refer https://grpc.io/docs/protoc-installation/

For MacOS,using Homebrew

brew install protobuf

protoc --version

## Application structure

User -> API Interface -> Protocol{http|grpc|job} -> Services{logs|metrics|traces|users} -> Infra{db|storage|config} -> meta | common{thirdparty|function utils} 

## init

1. register cluster
2. load cache
3. start move job
4. start file list
5. start grpc
6. start http
7. change node status to online

## Format code

```
cargo clippy -- -A clippy::uninlined_format_args
```

## check cargo build feature

```
diff -u <(rustc --print cfg) <(rustc -C target-cpu=native --print cfg)
```

# How to develop & contribute to Zinc

Check the [contributing guide](./CONTRIBUTING.md)
