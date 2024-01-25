cd out
w=$(basename -- ./main.wasm)
p=./main.wasm 
echo "Minifying $w, make sure it is not stripped"
wasm-snip $p --snip-rust-fmt-code --snip-rust-panicking-code -p core::num::flt2dec::.* -p core::fmt::float::.*  \
    --output temp-$w
wasm-gc temp-$w
wasm-strip temp-$w
wasm-opt -Oz temp-$w --output minified.wasm
rm temp-$w
echo $w `stat -c "%s" $p` "bytes ->" `stat -c "%s" minified.wasm` "bytes, see minified.wasm"
