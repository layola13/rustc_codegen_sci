#include <stdint.h>

struct OneI8 {
    int8_t value;
};

struct OneU8 {
    uint8_t value;
};

struct OneI16 {
    int16_t value;
};

struct OneU16 {
    uint16_t value;
};

struct OneI32 {
    int32_t value;
};

struct OneU32 {
    uint32_t value;
};

struct OneI64 {
    int64_t value;
};

struct OneU64 {
    uint64_t value;
};

extern struct OneI8 sci_abi_cast_return_i8(int8_t value);
extern struct OneU8 sci_abi_cast_return_u8(uint8_t value);
extern struct OneI16 sci_abi_cast_return_i16(int16_t value);
extern struct OneU16 sci_abi_cast_return_u16(uint16_t value);
extern struct OneI32 sci_abi_cast_return_i32(int32_t value);
extern struct OneU32 sci_abi_cast_return_u32(uint32_t value);
extern struct OneI64 sci_abi_cast_return_i64(int64_t value);
extern struct OneU64 sci_abi_cast_return(uint64_t value);

int main(void) {
    struct OneI8 result_i8 = sci_abi_cast_return_i8((int8_t)-42);
    if (result_i8.value != (int8_t)-42) {
        return 1;
    }

    struct OneU8 result_u8 = sci_abi_cast_return_u8(UINT8_C(0xa5));
    if (result_u8.value != UINT8_C(0xa5)) {
        return 2;
    }

    struct OneI16 result_i16 = sci_abi_cast_return_i16((int16_t)-1234);
    if (result_i16.value != (int16_t)-1234) {
        return 3;
    }

    struct OneU16 result_u16 = sci_abi_cast_return_u16(UINT16_C(0xa55a));
    if (result_u16.value != UINT16_C(0xa55a)) {
        return 4;
    }

    struct OneI32 result_i32 = sci_abi_cast_return_i32(INT32_C(-12345678));
    if (result_i32.value != INT32_C(-12345678)) {
        return 5;
    }

    struct OneU32 result_u32 = sci_abi_cast_return_u32(UINT32_C(0xa55a1234));
    if (result_u32.value != UINT32_C(0xa55a1234)) {
        return 6;
    }

    struct OneI64 result_i64 = sci_abi_cast_return_i64(-INT64_C(123456789012345));
    if (result_i64.value != -INT64_C(123456789012345)) {
        return 7;
    }

    struct OneU64 result = sci_abi_cast_return(UINT64_C(0x1122334455667788));
    if (result.value != UINT64_C(0x1122334455667788)) {
        return 8;
    }
    return 0;
}
