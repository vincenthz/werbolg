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

* Value Allocator - Exec
* Tail Call Optimisation (TCO) - Exec/Compile
* IR Namespacing - Frontends/Compile
