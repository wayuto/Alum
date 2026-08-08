export codes=$(find . -type f -regextype posix-extended -regex ".*\.(al|c|cpp)$")
export compile_al="alc bubblesort.al -o bubble_al"
export compile_cpp="g++ bubblesort.cpp -o bubble_cpp -O3 -std=c++17"

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
ls -l bubble_*
echo

echo "========== Running =========="
echo "Alum:"
./bubble_al
echo "C++:"
./bubble_cpp
echo

echo "========== Benchmark =========="

hyperfine "./bubble_cpp" "./bubble_al" --warmup 10 -i --shell none
rm bubble_*
