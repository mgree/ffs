#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>

int main(int argc, char *argv[]) {
  char *path;

  if (argc == 1) {
    path = ".";
  } else if (argc == 2) {
    path = argv[1];
  } else if (argc > 2) {
    fprintf(stderr, "Usage: %s [path]\n", argv[0]);
    return 1;
  }

  int fd = open(path, O_RDONLY);
  if (fd == -1) {
    perror(argv[0]);
    return 2;
  }

  fsync(fd);
  close(fd);

  return 0;
}
