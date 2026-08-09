export codes=$(find . -type f -regextype posix-extended -regex ".*\.(al|c|cpp|rs|zig|py)$")
export compile_al="alc fib40.al -o fib_al"
export compile_cpp="g++ fib40.cpp -o fib_cpp -O3 -fconstexpr-ops-limit=331999999"
export compile_rust="rustc -O fib40.rs -o fib_rust"
export compile_zig="zig build-exe fib40.zig -O ReleaseFast -femit-bin=fib_zig"

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
ls -l fib_*
echo

echo "========== Running =========="
echo "Alum: $(./fib_al)"
echo "C++: $(./fib_cpp)"
echo "Rust: $(./fib_rust)"
echo "Zig: $(./fib_zig)"
echo "Python: $(python3 fib40.py)"
echo

echo "========== Benchmark =========="

hyperfine "./fib_cpp" "./fib_al" "./fib_rust" "./fib_zig" "python3 fib40.py" --warmup 10 -m 5 -i --shell none
rm fib_*