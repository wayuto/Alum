$ifndef ALUM_IO
$define ALUM_IO 1

extern write(int, string, int): int
extern read(int, string, int): int
extern print(string): int
extern println(string): int
extern input(string): string
extern fopen(string, int, int): int
extern fclose(int): int
extern fread(int): string
extern fwrite(int, string, int): int
extern lseek(int, int, int): int

$endif