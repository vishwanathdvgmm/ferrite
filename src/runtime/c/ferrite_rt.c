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

extern void ferrite_main();

int main() {
  ferrite_main();
  return 0;
}
