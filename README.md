# prost-serde-derive

`prost-serde-derive` is a procedural macro to generate [Serde] serializers and deserializers for [Prost]-generated structs.

## Rationale

Currently [Prost] does not support JSON serialization-deserialization. Although we have the almighty [Serde] for JSON serialization and deserialization, using `serde_derive` is not possible for Prost-generated structs because representation of enumerations is different between Prost structs and [Protobuf-JSON](https://developers.google.com/protocol-buffers/docs/reference/java/com/google/protobuf/util/JsonFormat) format. Prost structs represent enumerations as plain integers but Protobuf-JSON represents as text identifers so using `serde_derive` won't work. This procedural macro is intended to fix the issue.

## Usage

In order to use, you need a Git master branch of `prost-build` which implements required methods for Prost enumerations for now. If you're using [tonic_build], there is a [fork](https://github.com/segfault87/tonic/tree/create-enum-from-str-name) for using Git version of `prost-build`.

To see it in action, see `example` crate.

## TODO

* Documentation
* Unit tests

[Serde]: https://serde.rs
[Prost]: https://github.com/tokio-rs/prost
[tonic_build]: https://github.com/hyperium/tonic