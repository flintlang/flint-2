# The State of Flint 2
_by Jessica Lally and George Stacey_

### Purpose
This report covers the state of the Flint 2 programming language─what has been implemented, how things are done, the current known issues, and any likely problems that might arise and have not been thoroughly investigated. It has been written in the hope that future developers may start with a better understanding of the project, and not waste as much time worrying about things that are broken.

### Contents
- Configuration
  - Installation
    - Dependencies
  - Testing
    - Libra
    - Ethereum
  - Development
    - IDEs
- Implementation
  - Move Translation
    - Reference Handling
  - eWASM Translation
    - LLVM Translation
      - Data Layout
      - ABI
      - Imports and Runtime Functions
      - Money
    - WASM to eWASM
- Known Issues
  - For-loops
  - Move
    - Dictionaries
    - Variable Mangling
  - eWASM
    - Arrays and Dictionaries
    - Unimplemented Expressions and Statements
- Likely Problems
  - Libra Updates

## Configuration
### Installation

Installation is fairly straightforward due to Rust's package management: simply clone the repository, install the dependencies and set up the flint_config file as detailed below, and run ```cargo build```. 

The flint_config.json file is specific to every user, detailing the path to your top-level Libra installation, your llc and your wasm-ld. If you have multiple versions of llc and/or wasm-ld you will also need to specify the version, and you may have to specify the path to the LLVM target using ```LLVM_SYS_100_PREFIX=<path_to_llvm-10.0> cargo build```. On Linux this path is likely to be ```/usr/lib/llvm-10```.

The compiler has currently only been tested on Ubuntu 18.04 and macOS 10.15.6 (//TODO: Matthew Fedora). Although it should be possible to install the pre-build binaries for LLVM for Windows, we were unable to do so.

_Note that we're going to cover the necessary dependencies for someone working on Flint, not someone using it. We believe the online instructions should be sufficient for people who wish to just use flintc_

#### Dependencies 

- **rustc**: We were working with version 1.44.1, however any newer installation is likely to work.
- **Libra**: Libra is known to update often without documentation, so there are likely to have been changes since writing this document that break parts of the Move target. The Move compiler currently works with the Libra version installed September 3rd 2020.
- **LLVM**: The Rust Inkwell crate currently supports up to LLVM 10.0, and some earlier versions of LLVM do not support the WASM target, hence this compiler is only tested with LLVM 10.0. You will also need to install wasm-ld-10 and llc-10.
- **python3**: In order to run the python script, you will need a python version >= 3.6.

### Testing
There are currently unit tests for the parser, some runtime function tests for LLVM and a semantic test of Move. These can be run using ```cargo test```. However, almost all of our testing is integration testing, split between compilation tests and runtime tests, as detailed below.

#### Libra
Travis tests the compilation of all Flint files to Move, however verification and behaviour testing of the Move files must be done locally, as Libra is too large a dependency to compile on Travis. To test the Libra target, run the python script with the command: ```python3 <path_to_python_file> <all | behaviour | compilation> Optional[<flint-filename-without-extension>]```.

_Note that the python file will take a very long time to run the first time as the script will need to compile Libra. Subsequent testing should be much faster._

#### Ethereum
The main impediment to our eWASM testing was that we were unable to set up an eWASM testnet. The official eWASM testnet is down at the time of writing, and we were not able to find a substitute. There are plenty of ethereum testnets out there for testing smart contracts, but very few of them support eWASM. This has meant that we simply have not been able to test the final eWASM. However, we have been able to test non-eWASM specific runtime functionality via LLVM testing. This is done simply by calling LLVM functions, and checking that they behave as expected. It allows us to test all aspects of generated code except that which relies on functions from the EEI. Furthermore, we have been able to verify that the WASM we produce is valid eWASM. This can all be done via ```cargo test``` and therefore is also tested by Travis upon every commit.

### Development
#### IDEs
Where possible, we would recommend using IntelliJ with the Rust plugins: Toml, Rust and Native debugging support (_Note at the time of writing Native debugging support is not supported for Windows_). Of course, it is possible to develop using any IDE of your choice which supports Rust.

## Implementation
### Move Translation
The initial Move compiler was built by Ali Chaudhry as part of his MEng thesis. However, due to Libra updates much of the compiler was out of date, and some features were not implemented, notably assertions, types states and caller protections. We also heavily refactored the codebase to comply with rustc clippy standards, and restructured modules into individual units. For the majority of the changes to the Move compiler, see [PR#13 Move stability](https://github.com/flintlang/flint-2/pull/13).

#### Reference Handling
Unlike Flint, which provides straight-up types and only requires references for passing structs as arguments, Move relies on strict reference control. The compiler references, copies and release objects in as controlled a way as possible, however the implementation was sometimes unreliable. This referencing system has been stabilised, and improved by eliminating double accesses (i.e. removing multiple temporary variables that reference the same data) and always converting the final copy of a variable to a move. There are still potential improvements to the reference handling system, such as only generating a mutable borrow when the reference is used mutably (i.e. it is used for another mutable borrow, or it is mutated).

### eWASM Translation
// TODO link pull request when it is written

The actual code generation is for LLVM, and we rely on the LLVM to wasm32 compiler to translate this correctly to WASM. From there we make some simple post-processing changes to the WASM in order to convert it to valid eWASM. We chose to compile to LLVM for the following reasons
- It is a well-established framework for creating a backend for programming languages and there exists a rust crate arounds the underlying C API, as well as many optimisations
- It has a wasm32 target, so all the difficulties of converting flint code to a stack based language
- The difficulties of WASM memory management are also delegated to LLVM
- It allows control over all imports and external linking, making it easy to ensure we only import from the ethereum namespace as required by the [ECI](https://ewasm.readthedocs.io/en/mkdocs/contract_interface/). This advantage was the main thing preventing us from compiling to a different intermediary such as C or AssemblyScript

_Note: For more information on why we chose to translate via LLVM, please see the [compiling flint to ewasm document](https://github.com/flintlang/flint-2/tree/eWASM/docs/papers/compiling_flint_to_ewasm.pdf)_

#### LLVM Translation
##### Data Layout
The contract declaration is converted to a struct type, alongside any other struct type declarations. We then create an instance of this contract struct, and store it as a global variable. This global variable then represents the state of the contract. Every function, should it wish to alter or view the state of the contract, simply loads from or to this global variable.

##### ABI
Ethereum contracts require the generation of an Application Binary Interface alongside the contract itself. This is simply a JSON string representing all publicly accessible functions and constructors. It includes information about them such as whether they are payable, the names and types of input parameters, return parameters etc. The ABI generation has mostly been implemented, certainly to the point where it should be usable for many projects. However, at the time of writing there are several things that are unimplemented, most notably implementations of all the different ethereum types (uint128, uint256, marking as payable etc.). This will need to be expanded for more complicated contracts, but it is not the highest priority, since most standard primitives that you might expect an external caller to interact with (bools, u64s etc.), are implemented.

##### Money
Currently, two runtime functions for handling money, ```Flint_balanceOf``` and ```Flint_transfer```, are implemented. These are standard library functions available globally in flint. They are wrappers around lower level LLVM runtime functions which interact directly with [EEI](https://ewasm.readthedocs.io/en/mkdocs/eth_interface/) functions. We are fairly confident that ```Flint_balanceOf_Inner```, which calls the eWASM function ```getExternalBalance```, has been implemented correctly. However, we have found very little documentation describing how money should be represented in eWASM. Since we do not yet know how money is represented, we are also unsure of how it should be transferred. We found a pull request on the eWASM repository which would, were it merged, offer a simple EEI function for transferring money between accounts. It does not appear that it is likely to be merged, so we commented on [the PR](https://github.com/ewasm/design/pull/113) asking how money transfers are supposed to be done. We hope that by the time future developers are working on this, there will be some updates on this. The current implementation of ```Flint_transfer_Inner``` has been based mainly on the Flint 1 implementation, using the ```call``` function to transfer money. Unfortunately, since we were unable to set up an eWASM testnet, we cannot be sure that our implementation of this function is correct, only that it is validated as correct eWASM.

Assets are currently unimplemented in eWASM, due to the confusion surrounding the representation of money described above.

#### WASM to eWASM
Once we have compiled to WASM, we need to make a few alterations to ensure we have generated valid eWASM. The specification for what constitutes valid eWASM can be found [here](https://ewasm.readthedocs.io/en/mkdocs/contract_interface/). The main points are: 
- Imports: Only imports from the ethereum namespace are allowed, where one may import [EEI](https://ewasm.readthedocs.io/en/mkdocs/eth_interface/) functions. This is taken care of throughout code generation, as if we use external functionality, we tell LLVM to link it according to these rules.
- Main function: There must exist a function that takes no parameters, and returns no values, exported under the name `main`. To satisfy this, we simply define such a function, which immediately returns. 
- No start function: There cannot be a function marked as a WASM entry function.
- Exports: There must be exactly two exports: `main` and `memory`. LLVM exports the memory when we generate the WASM, and it also exports all functions that we create. Since we created a dummy `main`, this is included and so we have both of these as exports. All that remains is to remove all the other exports which are not allowed. This is done by using a rust crate wrapper around [WABT](https://github.com/WebAssembly/wabt) to translate the generated WASM file to the human readable WAT file. We can then use regular expressions to remove all exports apart from the main and memory exports. We then convert it back to WASM, and at this point we should have valid eWASM. 

## Adding targets
All sections of the compiler except the final code generation should be target agnostic. The parser, type checking, semantic analysis etc. must therefore have no references to libra or ethereum or any other blockchain specific concepts. The system for extending flint to more targets is as follows (we will refer to the imaginary blockchain we are adding as 'popcorn'):
#### Target
Each target is an instantiation of the Target struct, which contains all the information specific to that target. This consists of:
- Its name 
- Its currencies
- Its preprocessor
- The function responsible for generating it given the parsed module and context
- A path to its standard library, written in flint

The first step in adding support for our popcorn blockchain is therefore to create a target for it, albeit with placeholders for the preprocessor and generate function  

#### Preprocessor
The preprocessor is for altering the flint code in ways that will make the final code generation more straightforward. For example, creating wrapper functions that can be called externally around flint functions might be done in the preprocessor. 
Anything like this can be done in the preprocessor. As a general rule of thumb, we suggest that if you find yourself inserting or altering AST structures, then this should be done in your target's preprocessor.

#### Code generation
Now your AST should be in a state where it can almost exactly be translated into the target language. How you do this is up to you and will depend on the target language. 

## Known Issues
In addition to the open issues in the Github repository, there are a number of other known issues outlined below. For a full list of currently unimplemented features, see the [guide](https://github.com/flintlang/flint-2/blob/eWASM/docs/guide.md).

### For-loops 
For-loops are currently unimplemented in both the Move and eWASM compiler. 

### Move
#### Dictionaries
Due to their current implementation, dictionaries are restricted to only having an Address key type. 

#### Variable Mangling
Currently variable mangling is not implemented. Consider any contract that has typestates. Since it is a stateful contract, when it is compiled to MoveIR or LLVM, the contract has an implicit field called `_contract_state`. This means that if a contract is written that has a variable called `_contract_state` in it, there may be variable conflicts. This applies to any scenario where a compiler generated identifier is created that could conceivably conflict with a user defined one. A mangling system whereby variable names are conditionally changed at compile time to avoid this should be implemented. 

### eWASM
#### Arrays and Dictionaries
The current implementations of both arrays and dictionaries in the eWASM compiler are fairly limited. Arrays are only stack-allocated, and currently only fixed-sized arrays are implemented. However, there is bounds checking for array accesses which will revert the execution of the contract in the event of an attempted out of bounds access. Dictionaries are represented as a stack-allocated array of structs containing key-value pairs, and are even more limited. The key type is currently restricted to only Int, Address and Bool (as the key and index are compared using ```build_int_compare```, and these types are converted to int in LLVM). Also, only fixed-sized dictionaries are implemented, and you cannot currently replace a key-value pair in the dictionary, only replace the value corresponding to the key. We would suggest a Hashmap as a better implementation of a dictionary in LLVM. Unfortunately we only have these fixed size data structures due to time contraints; it may take some time to develop dynamically sized data types in LLVM, and will require heap memory management. Obviously, due to the permanence of a published smart contract, avoiding memory leaks will be essential.

#### Unimplemented Expressions and Statements
Currently, not all expression and statement types in the AST are implemented in eWASM, for example range expressions and do-catch statements.

## Likely Problems
### Libra Updates
Libra is (at the time of writing) still in early development, hence is known to update often (and often without documentation). This means that changes to Libra are likely to break parts of the MoveIR compiler, resulting in failing tests. We recommend looking at Libra's functional tests for the language, as these will show what they've had to change to keep their own tests passing, which should be a rough guide for fixing any issues. 

### eWASM runtime
As mentioned, we were unable to test the final eWASM code. It is therefore possible that there will be problems integrating the eWASM EEI functions, or running as a smart contract rather than a local program. We hope that these problems will be fairly minor, as the overwhelming majority of that which is implemented is testable via the LLVM. 
