# werbolg

werbolg is an interpreted execution engine

## Description

`werbolg` is very similar to `python` in spirit, except it allow to configure
your own variants for domain specific cases, with a system in rust and
extensible in rust.

Before execution, the ExecutionMachine need to be configured with native
functions that will allow the user to do specific actions, and hence the
ability to escape the werbolg sandbox is driven only by the user configuration.

By default the sandbox is completely empty of operations, and thus cannot
manipulate any values but only pass them around. Whilst it is not a useful
setup, it illustrates the security default of the model.

## TODO

* Struct support in lispy and rusty
* Enum in lispy and rusty and Compile
* Pattern match - Frontend/Compile/Exec
* Value Allocator - Exec
* Closure - Exec/Core
* Binary serialization for core - Core
* Binary serialization for instructions - Compile

## Architecture

```
┌───────────┐
│           │
│           ├───────────┐
│           │           │
│           │           │
│           │           ▼
│           │     ┌──────────┐   ┌──────────┐   ┌──────┐
│           │     │          │   │          │   │      │
│Frontends  ├────►│ Core AST ├──►│ Compiler ├──►│ Exec │
│           │     │          │   │          │   │      │
│ * lispy   │     └──────────┘   └──────────┘   └──────┘
│ * rusty   │           ▲
│ * ...     │           │
│           │           │
│           ├───────────┘
│           │
└───────────┘
```


## Tales

Basic way to test parsing and execution is using `werbolg-tales`

```
cargo run --bin werbolg-tales <TESTFILE>
```

By default `werbolg-tales` will try to auto-detect the type of file, from the supported frontend (rusty or lispy).
You can specify the backend explicitely using the frontend flags

```
cargo run --bin werbolg-tales -- --frontend rusty test.rusty
```


## Exec & Compile

Compile turns the IR into a very basic set of instructions that Exec will run.
Exec works like a simple [stack machine](https://en.wikipedia.org/wiki/Stack_machine)

The list of instructions for this stack machine can be found in instructions.rs
