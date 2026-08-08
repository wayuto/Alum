export codes=$(find . -type f -regextype posix-extended -regex ".*\.(al|c|cpp)$")
export compile_al="alc fib40.al -o fib_al"
export compile_cpp="g++ fib40.cpp -o fib_cpp -O3 -fconstexpr-ops-limit=331999999"

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

echo "========== Result =========="
ls -l fib_*
echo

echo "========== Running =========="
echo "Alum: $(./fib_al)"
echo "C++: $(./fib_cpp)"
echo

echo "========== Benchmark =========="

hyperfine "./fib_cpp" "./fib_al" --warmup 10 -i --shell none
rm fib_*