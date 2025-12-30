al -c bubble_sort.al
gcc -O3 bubble_sort.c
hyperfine -i './bubble_sort' './a.out' 'python bubble_sort.py' --shell=none --warmup 100
rm bubble_sort a.out