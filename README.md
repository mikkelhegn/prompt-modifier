# Running untrusted code in a sandboxed environment

I want to show the following:

1. You can compile Python code, written against a defined interface, which also does static analysis of the Wasm component.
1. The compiled component can be composed in an existing app (LLM pre-processing and post-processing)
1. The app can package and deploy the application to a cloud
1. You can run and use the app in the cloud

## Flow

1. Write a simple function (takes a string, returns a string)
   1. Write a test, shows it works...

2. Wire it up with WIT-bindings
3. Build the component
4. Validate WIT imports
5. Test? (run in host?)
6. Compose
7. Test? (spin-test)
8. Deploy Spin app

1-2 bring a component

1. Build WASM from Python
   - An api to execute `componentize-py`
   - Validate the Wasm component
2. Test (This may not make sense)
   - run wasm and check returned string
3. Compose
   - use WAC to compose with existing Spin component
     - wac targets - Determines whether a given component conforms to the supplied wit world.
     - wac parse - Parses a composition into a JSON representation of the AST.
     - wac resolve - Resolves a composition into a JSON representation.
   - test Spin app

---

4. Deploy and return endpoint

---

5. Take .wasm as input is an extended feature

---

Directory structure:
README.md
spin-app-template
The template for the end result
python-examples
The example for writing a Python component
js-example
The example of wirtihgn a js component
processor
The app that processes the whole thing
shared
Anything needed to be shared among all the above

---

## Pre-reqs

https://github.com/bytecodealliance/componentize-py#getting-started
`pip install componentize-py`

## Discoveries

### Componentize-py not supporting reference to Python module using `../test/python`

```shell
> componentize-py -d ../test/wit/world.wit -w string-processor componentize --stub-wasi '../test/python' -o componentb.wasm

Traceback (most recent call last):
  File "/Users/mikkel/code/temp/python/venv/bin/componentize-py", line 8, in <module>
    sys.exit(script())
             ^^^^^^^^
AssertionError: ModuleNotFoundError: No module named '.'


Caused by:
    ModuleNotFoundError: No module named '.'
```

### Componentize-py support for no WASI interfaces

Stub wasi seems to be the option that is actually fixing this.
Current work around is to use wasi-virt (howver this only supports WASI@0.2.1, and Componentize-py is at @0.2.0). Probably good anyway as this will be used to remove any interface calls not allowed.

### Spin deps "hack"

https://github.com/KaiWalter/wasm-api-gateway/pull/1/files

### Converting between casing is a nightmare...

Looks like wit-bindgen (or spin deps) got it wrong with something that had two dashes - e.g. "string-processor-processing-strings"
