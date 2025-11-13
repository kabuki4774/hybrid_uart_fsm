#include <stdint.h>
#include <stdio.h>

extern int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size);

int main(void)
{
    static const uint8_t sample1[] = {0xAA, 0x04, 0x01, 0x02, 0xF8};
    static const uint8_t sample2[] = {0xAA, 0x05, 0x03, 'h', 'i', 0xF9};

    puts("Running fuzz samples manually...");
    LLVMFuzzerTestOneInput(sample1, sizeof(sample1));
    LLVMFuzzerTestOneInput(sample2, sizeof(sample2));

    puts("Done.");
    return 0;
}
