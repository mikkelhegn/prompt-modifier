# Securely running un-trusted code using WebAssembly and Rust

This ia sample showcasing the following:

1. You can compile Python code, written against a defined interface
1. The compiled component can be composed in an existing app (LLM pre-processing and post-processing)
1. The app can package and deploy the application to a cloud
1. You can run and use the app in the cloud

## Trying out

Python example

```
./target/release/string-processor --source ../examples/python/ --language python
mv composed.wasm ../spin-app-template/
cd ../spin-app-template/
spin up
```

Wasm example

```
./target/release/string-processor --source ../examples/js/promptmodifierJS.wasm
mv composed.wasm ../spin-app-template/
cd ../spin-app-template/
spin up
```
