#include <stdio.h>

int main(int argc, char **argv) {
  if (argc < 3) {
    puts("Usage: clog <input dir> <output dir>");
    return 1;
  }
  char *input_dir = argv[1];
  char *output_dir = argv[2];
  printf("input dir: %s\n", input_dir);
  printf("output dir: %s\n", output_dir);
  return 0;
}
