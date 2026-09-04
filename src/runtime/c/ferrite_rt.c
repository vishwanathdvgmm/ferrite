#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Ferrite String Representation matches the LLVM struct { i8* ptr, i64 len }
typedef struct {
  char *ptr;
  int64_t len;
} FerriteString;

#define ARENA_SIZE 1024 * 1024 * 64 // 64 MB arena for strings
static char string_arena[ARENA_SIZE];
static size_t arena_offset = 0;

void *ferrite_alloc(size_t size) {
  if (arena_offset + size > ARENA_SIZE) {
    // Fallback to malloc if arena is full
    return malloc(size);
  }
  void *ptr = string_arena + arena_offset;
  arena_offset += size;
  return ptr;
}

// str1 + str2
FerriteString *ferrite_string_concat(char *a_ptr, int64_t a_len, char *b_ptr,
                                     int64_t b_len) {
  int64_t new_len = a_len + b_len;
  FerriteString *result =
      (FerriteString *)ferrite_alloc(sizeof(FerriteString) + new_len + 1);
  if (!result) {
    fprintf(stderr, "Ferrite Runtime Error: Out of memory\n");
    exit(1);
  }

  char *new_ptr = (char *)(result + 1);
  if (a_len > 0 && a_ptr)
    memcpy(new_ptr, a_ptr, a_len);
  if (b_len > 0 && b_ptr)
    memcpy(new_ptr + a_len, b_ptr, b_len);
  new_ptr[new_len] = '\0';

  result->ptr = new_ptr;
  result->len = new_len;
  return result;
}

// str(int)
FerriteString *ferrite_int_to_string(int64_t val) {
  char buffer[32];
  int len = snprintf(buffer, sizeof(buffer), "%lld", (long long)val);

  FerriteString *result =
      (FerriteString *)ferrite_alloc(sizeof(FerriteString) + len + 1);
  if (!result) {
    fprintf(stderr, "Ferrite Runtime Error: Out of memory\n");
    exit(1);
  }

  char *new_ptr = (char *)(result + 1);
  memcpy(new_ptr, buffer, len + 1);

  result->ptr = new_ptr;
  result->len = len;
  return result;
}

// str(float)
FerriteString *ferrite_float_to_string(double val) {
  char buffer[64];
  int len = snprintf(buffer, sizeof(buffer), "%f", val);

  FerriteString *result =
      (FerriteString *)ferrite_alloc(sizeof(FerriteString) + len + 1);
  if (!result) {
    fprintf(stderr, "Ferrite Runtime Error: Out of memory\n");
    exit(1);
  }

  char *new_ptr = (char *)(result + 1);
  memcpy(new_ptr, buffer, len + 1);

  result->ptr = new_ptr;
  result->len = len;
  return result;
}

// println(str)
void ferrite_println(char *ptr, int64_t len) {
  if (len > 0 && ptr) {
    fwrite(ptr, 1, len, stdout);
  }
  printf("\n");
}

// print(str)
void ferrite_print(char *ptr, int64_t len) {
  if (len > 0 && ptr) {
    fwrite(ptr, 1, len, stdout);
  }
}

// ==========================================
// Stdlib IO & String Bindings
// ==========================================

FerriteString *__builtin_io_read_file(FerriteString *path_str) {
  char path[1024];
  int64_t len = path_str->len < 1023 ? path_str->len : 1023;
  memcpy(path, path_str->ptr, len);
  path[len] = '\0';

  FILE *f = fopen(path, "rb");
  if (!f)
    return ferrite_string_concat(NULL, 0, NULL, 0); // Empty string on error
  fseek(f, 0, SEEK_END);
  int64_t fsize = ftell(f);
  fseek(f, 0, SEEK_SET);

  FerriteString *result =
      (FerriteString *)ferrite_alloc(sizeof(FerriteString) + fsize + 1);
  char *new_ptr = (char *)(result + 1);
  fread(new_ptr, 1, fsize, f);
  fclose(f);

  new_ptr[fsize] = '\0';
  result->ptr = new_ptr;
  result->len = fsize;
  return result;
}

void __builtin_io_write_file(FerriteString *path_str,
                             FerriteString *content_str) {
  char path[1024];
  int64_t len = path_str->len < 1023 ? path_str->len : 1023;
  memcpy(path, path_str->ptr, len);
  path[len] = '\0';

  FILE *f = fopen(path, "wb");
  if (f) {
    fwrite(content_str->ptr, 1, content_str->len, f);
    fclose(f);
  }
}

void __builtin_io_append_file(FerriteString *path_str,
                              FerriteString *content_str) {
  char path[1024];
  int64_t len = path_str->len < 1023 ? path_str->len : 1023;
  memcpy(path, path_str->ptr, len);
  path[len] = '\0';

  FILE *f = fopen(path, "ab");
  if (f) {
    fwrite(content_str->ptr, 1, content_str->len, f);
    fclose(f);
  }
}

int64_t __builtin_io_file_exists(FerriteString *path_str) {
  char path[1024];
  int64_t len = path_str->len < 1023 ? path_str->len : 1023;
  memcpy(path, path_str->ptr, len);
  path[len] = '\0';

  FILE *f = fopen(path, "rb");
  if (f) {
    fclose(f);
    return 1;
  }
  return 0;
}

// NOTE: String functions that return Lists (e.g., split) are complex in C
// because they require constructing Ferrite List structs (which contain
// RefCells in interpreter, but bare structs in LLVM). For complete full LLVM
// support of strings, these bindings can be implemented here. For brevity, a
// simple substring implementation is shown.

FerriteString *__builtin_string_substr(FerriteString *s, int64_t start,
                                       int64_t length) {
  if (start < 0)
    start = 0;
  if (length < 0)
    length = 0;
  if (start > s->len)
    start = s->len;
  if (start + length > s->len)
    length = s->len - start;

  return ferrite_string_concat(s->ptr + start, length, NULL, 0);
}

extern void ferrite_main();

int main() {
  ferrite_main();
  return 0;
}
