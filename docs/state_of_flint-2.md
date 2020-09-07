# The State of Flint-2
_by Jessica Lally and George Stacey_

### Purpose
This report covers the state of the Flint programming language─what has been implemented, how things are done, the current known issues, and any likely problems that might arise and have not been thoroughly investigated. It has been written in the hope that future developers may start with a better understanding of the project, and not waste as much time worrying about things that are broken.

### Contents
// TODO: contents
- Configuration
  - Installation
    - Dependencies
  - Testing
    - Libra
    - Ethereum
  - Development
    - IDEs
- Implementation
  - MoveIR Translation
  - eWASM Translation

## Configuration
### Installation

Installation is fairly straightforward due to rust package management, simply clone the repository, install the dependencies and set up the flint_config file as detailed below, and run ```cargo build```. 

The flint_config.json file is specific to every user, detailing the path to your top-level Libra installation, your llc and your wasm-ld. If you have multiple versions of llc and/or wasm-ld you will also need to specify the version, and you may have to specify the path to the LLVM target using ```LLVM_SYS_100_PREFIX=<path_to_llvm-10.0> cargo build```. On Linux this path is likely to be ```/usr/lib/llvm-10```.

The compiler has currently only been tested on Ubuntu 18.04 and macOS 10.15.6 (//TODO: Fedora?). Although it should be possible to install the pre-build binaries for LLVM for Windows, we were unable to do so.

_Note that we're going to cover the necessary dependencies for someone working on Flint, not someone using it. We believe the online instructions should be sufficient for people who wish to just use flintc_

#### Dependencies 

- **rustc**: We were working with version 1.44.1, however any newer installation is likely to work.
- **Libra**: Libra is known to update often without documentation, so there are likely to have been changes since writing this document that break parts of the MoveIR target. The MoveIR compiler currently works with the Libra version installed September 3rd 2020.
- **LLVM**: The rust Inkwell crate currently supports up to LLVM 10.0, and some earlier versions of LLVM do not support the WASM target, hence this compiler is only tested with LLVM 10.0. You will also need to install wasm-ld and llc.
- **python3**: In order to run the python script, you will need a python version >= 3.6.

### Testing
There are currently unit tests for the parser, some runtime function tests for LLVM and a semantic test of MoveIR. These can be run using ```cargo test```. However, almost all of our testing is integration testing, split between compilation tests and runtime tests, as detailed below.

#### Libra
Travis tests the compilation of all Flint files to MoveIR, however verification and behaviour testing of the MoveIR files should be done locally, as Libra is too large a dependency to compile on Travis. To test the Libra target, run the python script with the command: ```python3 <path_to_python_file> <all | behaviour | compilation> Optional[<flint-filename-without-extension>]```.

_Note that the python file will take a long time to run the first time as the script will need to compile Libra. Subsequent testing should be much faster._

#### Ethereum
We were not able to get the eWASM testnet working, so we haven't been able to test the generated eWASM. However, we have been able to test non-eWASM specific runtime functionality via LLVM testing, and we have been able to verify that the WASM we produce is valid eWASM. This is all done via ```cargo test``` and therefore is also tested by Travis.

### Development
#### IDEs
Where possible, we would recommend using IntelliJ with the Rust plugin: Toml, Rust and Native debugging support (_Note at the time of writing Native debugging support is not supported for Windows_). Of course, it is possible to develop using any IDE of your choice which supports Rust.

## Implementation
### MoveIR Translation
The initial MoveIR compiler was built by Ali Chaudhry as part of his MEng thesis. However, due to Libra updates much of the compiler was out of date, and some features were not implemented, notably assertions, types states and caller protections. We also heavily refactored the code base to comply with rustc clippy standards, and restructured modules into individual units. For the majority of the changes to the MoveIR compiler, see [PR#13 Move stability] (https://github.com/flintlang/flint-2/pull/13).

#### Reference Handling
// TODO: Jess

### eWASM Translation
The actual code generation is for LLVM, and we rely on the LLVM to WASM-32 compiler to translate this correctly to WASM. From there we make some simple post-processing changes to the WASM in order to convert it to valid eWASM. 

#### LLVM Translation
// TODO
##### Data Layout
We represent the contract data in a stack-allocated global variable which is a pointer to a struct which corresponds to the Flint contract declaration. 

##### ABI
// TODO: George

##### Imports and Runtime Functions

##### Money
// TODO: Jess

#### WASM to eWASM
// TODO: George

## Known Issues
In addition to the open issues in the github repository, there are a number of other known issues outlined below.

### For-loops 
For-loops are currently unimplemented in both the MoveIR and eWASM compiler. 

### Compiler Checks
// TODO: George

### MoveIR
#### Dictionaries

#### Variable Mangling
// TOD: George

### eWASM
#### Arrays and Dictionaries

## Likely Problems
### Libra Updates

