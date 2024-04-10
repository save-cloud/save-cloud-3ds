#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>

char *get_format_time() {
  char *str = malloc(24);
  struct timeval tv;
  gettimeofday(&tv, NULL);
  time_t t = time(NULL);
  struct tm *lt = localtime(&t);
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wformat-truncation="
  snprintf(str, 24, "%04d-%02d-%02d %02d.%02d.%02d.%03ld", lt->tm_year + 1900,
           lt->tm_mon + 1, lt->tm_mday, lt->tm_hour, lt->tm_min, lt->tm_sec,
           tv.tv_usec / 1000);
#pragma GCC diagnostic pop
  return str;
}

void free_c_str(char *str) { free(str); }
