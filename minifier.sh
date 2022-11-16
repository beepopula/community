cd out
for p in ./base/*.wasm ; do
  w=$(basename -- $p)
  ../minify.sh $p
  cp $p stripped-$w
  wasm-strip stripped-$w
  echo $w `stat -c "%s" stripped-$w` " -> " `stat -c "%s" minified-$w`
done