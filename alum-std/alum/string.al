$ifndef ALUM_STRING
$define ALUM_STRING 1

extern strlen(string): int
extern strcpy(string, string): string
extern strcat(string, string): string
extern memcpy(string, string, int): string
extern memset(string, int, int): string
extern bcmp(string, string, int): int
extern memcmp(string, string, int): int

$endif