# Fuzzing

Install `cargo-fuzz`:

```sh
cargo install -f cargo-fuzz
```

Run any available target where `$target` is the name of the target and `$n` is
the number of CPUs to use for fuzzing:

```sh
cargo fuzz list # get list of targets
cargo fuzz run $target -j $n
```
