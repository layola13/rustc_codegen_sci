#include <stdint.h>

struct OneU64 {
    uint64_t value;
};

extern struct OneU64 sci_abi_cast_return(uint64_t value);

int main(void) {
    struct OneU64 result = sci_abi_cast_return(UINT64_C(0x1122334455667788));
    if (result.value != UINT64_C(0x1122334455667788)) {
        return 1;
    }
    return 0;
}
