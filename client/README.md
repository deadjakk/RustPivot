# RustPivot Implant  

### Building  
1. First edit the 'port' and 'ip' variables in src/main.rs to match the backend
address configured for server.
2. Then build with: 
`cargo build --release --bin implant`
3. Then run with `./implant` or `.\implant.exe` (Windows)  

### Usage Tips  
- After building, run `strip` against the resulting binary to significantly reduce the binary size.   
- Look into using LTO and optimization for size to further reduce binary size.  
- It will build for whatever operating system it is compiled on unless
specified otherwise via the `--target` flag.  
