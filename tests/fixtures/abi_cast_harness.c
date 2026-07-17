#include <stdint.h>

struct OneU8 {
    uint8_t value;
};

struct OneU16 {
    uint16_t value;
};

struct OneU32 {
    uint32_t value;
};

struct OneU64 {
    uint64_t value;
};

extern struct OneU8 sci_abi_cast_return_u8(uint8_t value);
extern struct OneU16 sci_abi_cast_return_u16(uint16_t value);
extern struct OneU32 sci_abi_cast_return_u32(uint32_t value);
extern struct OneU64 sci_abi_cast_return(uint64_t value);

int main(void) {
    struct OneU8 result_u8 = sci_abi_cast_return_u8(UINT8_C(0xa5));
    if (result_u8.value != UINT8_C(0xa5)) {
        return 1;
    }

    struct OneU16 result_u16 = sci_abi_cast_return_u16(UINT16_C(0xa55a));
    if (result_u16.value != UINT16_C(0xa55a)) {
        return 2;
    }

    struct OneU32 result_u32 = sci_abi_cast_return_u32(UINT32_C(0xa55a1234));
    if (result_u32.value != UINT32_C(0xa55a1234)) {
        return 3;
    }

    struct OneU64 result = sci_abi_cast_return(UINT64_C(0x1122334455667788));
    if (result.value != UINT64_C(0x1122334455667788)) {
        return 4;
    }
    return 0;
}
