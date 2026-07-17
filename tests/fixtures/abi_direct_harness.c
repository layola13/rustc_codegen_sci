#include <stdint.h>

extern int8_t sci_abi_i8_add(int8_t a, int8_t b);
extern uint8_t sci_abi_u8_xor(uint8_t a, uint8_t b);
extern int16_t sci_abi_i16_sub(int16_t a, int16_t b);
extern uint16_t sci_abi_u16_or(uint16_t a, uint16_t b);
extern int32_t sci_abi_i32_mul(int32_t a, int32_t b);
extern uint32_t sci_abi_u32_and(uint32_t a, uint32_t b);
extern int64_t sci_abi_i64_add(int64_t a, int64_t b);
extern uint64_t sci_abi_u64_sub(uint64_t a, uint64_t b);
extern intptr_t sci_abi_isize_gt(intptr_t a, intptr_t b);
extern uintptr_t sci_abi_usize_add(uintptr_t a, uintptr_t b);
extern const int32_t *sci_abi_ptr_identity(const int32_t *value);
extern void sci_abi_void_noop(int32_t value);

extern int8_t sci_abi_call_host_i8_sub(int8_t a, int8_t b);
extern uint8_t sci_abi_call_host_u8_xor(uint8_t a, uint8_t b);
extern int16_t sci_abi_call_host_i16_add(int16_t a, int16_t b);
extern uint16_t sci_abi_call_host_u16_or(uint16_t a, uint16_t b);
extern int32_t sci_abi_call_host_i32_mul(int32_t a, int32_t b);
extern uint32_t sci_abi_call_host_u32_and(uint32_t a, uint32_t b);
extern int64_t sci_abi_call_host_i64_sub(int64_t a, int64_t b);
extern uint64_t sci_abi_call_host_u64_add(uint64_t a, uint64_t b);
extern intptr_t sci_abi_call_host_isize_gt(intptr_t a, intptr_t b);
extern uintptr_t sci_abi_call_host_usize_mul(uintptr_t a, uintptr_t b);
extern const int32_t *sci_abi_call_host_ptr_identity(const int32_t *value);
extern int32_t sci_abi_call_host_note_i32(int32_t value);

int8_t sci_host_abi_i8_sub(int8_t a, int8_t b) {
    return (int8_t)(a - b);
}

uint8_t sci_host_abi_u8_xor(uint8_t a, uint8_t b) {
    return (uint8_t)(a ^ b);
}

int16_t sci_host_abi_i16_add(int16_t a, int16_t b) {
    return (int16_t)(a + b);
}

uint16_t sci_host_abi_u16_or(uint16_t a, uint16_t b) {
    return (uint16_t)(a | b);
}

int32_t sci_host_abi_i32_mul(int32_t a, int32_t b) {
    return a * b;
}

uint32_t sci_host_abi_u32_and(uint32_t a, uint32_t b) {
    return a & b;
}

int64_t sci_host_abi_i64_sub(int64_t a, int64_t b) {
    return a - b;
}

uint64_t sci_host_abi_u64_add(uint64_t a, uint64_t b) {
    return a + b;
}

intptr_t sci_host_abi_isize_gt(intptr_t a, intptr_t b) {
    return a > b ? (intptr_t)1 : (intptr_t)0;
}

uintptr_t sci_host_abi_usize_mul(uintptr_t a, uintptr_t b) {
    return a * b;
}

const int32_t *sci_host_abi_ptr_identity(const int32_t *value) {
    return value;
}

static int32_t host_note_total = 0;

void sci_host_abi_note_i32(int32_t value) {
    host_note_total += value;
}

int main(void) {
    static const int32_t ptr_probe = 1234;

    if (sci_abi_i8_add((int8_t)12, (int8_t)30) != (int8_t)42) {
        return 1;
    }
    if (sci_abi_u8_xor((uint8_t)0x3c, (uint8_t)0x16) != (uint8_t)42) {
        return 2;
    }
    if (sci_abi_i16_sub((int16_t)100, (int16_t)58) != (int16_t)42) {
        return 3;
    }
    if (sci_abi_u16_or((uint16_t)0x0028, (uint16_t)0x0002) != (uint16_t)42) {
        return 4;
    }
    if (sci_abi_i32_mul(6, 7) != 42) {
        return 5;
    }
    if (sci_abi_u32_and(UINT32_C(0xff2a), UINT32_C(0x2a)) != UINT32_C(42)) {
        return 6;
    }
    if (sci_abi_i64_add(INT64_C(40), INT64_C(2)) != INT64_C(42)) {
        return 7;
    }
    if (sci_abi_u64_sub(UINT64_C(100), UINT64_C(58)) != UINT64_C(42)) {
        return 8;
    }
    if (sci_abi_isize_gt((intptr_t)-2, (intptr_t)-5) != (intptr_t)1) {
        return 9;
    }
    if (sci_abi_usize_add((uintptr_t)39, (uintptr_t)3) != (uintptr_t)42) {
        return 10;
    }
    if (sci_abi_ptr_identity(&ptr_probe) != &ptr_probe) {
        return 11;
    }
    sci_abi_void_noop(42);

    if (sci_abi_call_host_i8_sub((int8_t)50, (int8_t)8) != (int8_t)42) {
        return 12;
    }
    if (sci_abi_call_host_u8_xor((uint8_t)0x30, (uint8_t)0x1a) != (uint8_t)42) {
        return 13;
    }
    if (sci_abi_call_host_i16_add((int16_t)20, (int16_t)22) != (int16_t)42) {
        return 14;
    }
    if (sci_abi_call_host_u16_or((uint16_t)0x0020, (uint16_t)0x000a) != (uint16_t)42) {
        return 15;
    }
    if (sci_abi_call_host_i32_mul(7, 6) != 42) {
        return 16;
    }
    if (sci_abi_call_host_u32_and(UINT32_C(0x7e), UINT32_C(0x2a)) != UINT32_C(42)) {
        return 17;
    }
    if (sci_abi_call_host_i64_sub(INT64_C(100), INT64_C(58)) != INT64_C(42)) {
        return 18;
    }
    if (sci_abi_call_host_u64_add(UINT64_C(19), UINT64_C(23)) != UINT64_C(42)) {
        return 19;
    }
    if (sci_abi_call_host_isize_gt((intptr_t)9, (intptr_t)3) != (intptr_t)1) {
        return 20;
    }
    if (sci_abi_call_host_usize_mul((uintptr_t)6, (uintptr_t)7) != (uintptr_t)42) {
        return 21;
    }
    if (sci_abi_call_host_ptr_identity(&ptr_probe) != &ptr_probe) {
        return 22;
    }
    if (sci_abi_call_host_note_i32(42) != 99) {
        return 23;
    }
    if (host_note_total != 42) {
        return 24;
    }

    return 0;
}
