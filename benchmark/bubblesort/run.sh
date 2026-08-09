export codes=$(find . -type f -regextype posix-extended -regex ".*\.(al|c|cpp|rs|zig|py)$")
export compile_al="alc bubblesort.al -o bubble_al"
export compile_cpp="g++ bubblesort.cpp -o bubble_cpp -O3 -std=c++17"
export compile_rust="rustc -O bubblesort.rs -o bubble_rust"
export compile_zig="zig build-exe bubblesort.zig -O ReleaseFast -femit-bin=bubble_zig"

for code in $codes
do
        echo
        echo "========= $code ==========="
        cat $code
        echo
done

echo "========== $compile_al =========="
time bash -c "$compile_al"
echo
echo "==========  $compile_cpp  =========="
time bash -c "$compile_cpp"
echo
echo "==========  $compile_rust  =========="
time bash -c "$compile_rust"
echo
echo "==========  $compile_zig  =========="
time bash -c "$compile_zig"
echo
echo "========== Result =========="
ls -l bubble_*
echo

echo "========== Running =========="
echo "Alum:"
./bubble_al
echo "C++:"
./bubble_cpp
echo "Rust:"
./bubble_rust
echo "Zig:"
./bubble_zig
echo "Python:"
python3 bubblesort.py
echo

echo "========== Benchmark =========="

hyperfine "./bubble_cpp" "./bubble_al" "./bubble_rust" "./bubble_zig" "python3 bubblesort.py" --warmup 10 -m 5 -i --shell none
rm bubble_*