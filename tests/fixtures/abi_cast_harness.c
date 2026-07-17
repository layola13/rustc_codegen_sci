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
extern int8_t sci_abi_cast_arg_i8(struct OneI8 value);
extern uint8_t sci_abi_cast_arg_u8(struct OneU8 value);
extern int16_t sci_abi_cast_arg_i16(struct OneI16 value);
extern uint16_t sci_abi_cast_arg_u16(struct OneU16 value);
extern int32_t sci_abi_cast_arg_i32(struct OneI32 value);
extern uint32_t sci_abi_cast_arg_u32(struct OneU32 value);
extern int64_t sci_abi_cast_arg_i64(struct OneI64 value);
extern uint64_t sci_abi_cast_arg_u64(struct OneU64 value);
extern int8_t sci_abi_call_host_cast_arg_i8(int8_t value);
extern uint8_t sci_abi_call_host_cast_arg_u8(uint8_t value);
extern int16_t sci_abi_call_host_cast_arg_i16(int16_t value);
extern uint16_t sci_abi_call_host_cast_arg_u16(uint16_t value);
extern int32_t sci_abi_call_host_cast_arg_i32(int32_t value);
extern uint32_t sci_abi_call_host_cast_arg_u32(uint32_t value);
extern int64_t sci_abi_call_host_cast_arg_i64(int64_t value);
extern uint64_t sci_abi_call_host_cast_arg_u64(uint64_t value);
extern int8_t sci_abi_call_host_cast_return_i8(int8_t value);
extern uint8_t sci_abi_call_host_cast_return_u8(uint8_t value);
extern int16_t sci_abi_call_host_cast_return_i16(int16_t value);
extern uint16_t sci_abi_call_host_cast_return_u16(uint16_t value);
extern int32_t sci_abi_call_host_cast_return_i32(int32_t value);
extern uint32_t sci_abi_call_host_cast_return_u32(uint32_t value);
extern int64_t sci_abi_call_host_cast_return_i64(int64_t value);
extern uint64_t sci_abi_call_host_cast_return_u64(uint64_t value);

int8_t sci_host_cast_arg_i8(struct OneI8 value) {
    return value.value;
}

uint8_t sci_host_cast_arg_u8(struct OneU8 value) {
    return value.value;
}

int16_t sci_host_cast_arg_i16(struct OneI16 value) {
    return value.value;
}

uint16_t sci_host_cast_arg_u16(struct OneU16 value) {
    return value.value;
}

int32_t sci_host_cast_arg_i32(struct OneI32 value) {
    return value.value;
}

uint32_t sci_host_cast_arg_u32(struct OneU32 value) {
    return value.value;
}

int64_t sci_host_cast_arg_i64(struct OneI64 value) {
    return value.value;
}

uint64_t sci_host_cast_arg_u64(struct OneU64 value) {
    return value.value;
}

struct OneI8 sci_host_cast_return_i8(int8_t value) {
    return (struct OneI8){ .value = value };
}

struct OneU8 sci_host_cast_return_u8(uint8_t value) {
    return (struct OneU8){ .value = value };
}

struct OneI16 sci_host_cast_return_i16(int16_t value) {
    return (struct OneI16){ .value = value };
}

struct OneU16 sci_host_cast_return_u16(uint16_t value) {
    return (struct OneU16){ .value = value };
}

struct OneI32 sci_host_cast_return_i32(int32_t value) {
    return (struct OneI32){ .value = value };
}

struct OneU32 sci_host_cast_return_u32(uint32_t value) {
    return (struct OneU32){ .value = value };
}

struct OneI64 sci_host_cast_return_i64(int64_t value) {
    return (struct OneI64){ .value = value };
}

struct OneU64 sci_host_cast_return_u64(uint64_t value) {
    return (struct OneU64){ .value = value };
}

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

    if (sci_abi_cast_arg_i8((struct OneI8){ .value = (int8_t)-17 }) != (int8_t)-17) {
        return 9;
    }
    if (sci_abi_cast_arg_u8((struct OneU8){ .value = UINT8_C(0xc7) }) != UINT8_C(0xc7)) {
        return 10;
    }
    if (sci_abi_cast_arg_i16((struct OneI16){ .value = (int16_t)-123 }) != (int16_t)-123) {
        return 11;
    }
    if (sci_abi_cast_arg_u16((struct OneU16){ .value = UINT16_C(0xcafe) }) != UINT16_C(0xcafe)) {
        return 12;
    }
    if (sci_abi_cast_arg_i32((struct OneI32){ .value = INT32_C(-9876543) }) != INT32_C(-9876543)) {
        return 13;
    }
    if (sci_abi_cast_arg_u32((struct OneU32){ .value = UINT32_C(0xcafe1234) }) != UINT32_C(0xcafe1234)) {
        return 14;
    }
    if (sci_abi_cast_arg_i64((struct OneI64){ .value = -INT64_C(9876543210123) }) != -INT64_C(9876543210123)) {
        return 15;
    }
    if (sci_abi_cast_arg_u64((struct OneU64){ .value = UINT64_C(0xfedcba9876543210) }) != UINT64_C(0xfedcba9876543210)) {
        return 16;
    }
    if (sci_abi_call_host_cast_arg_i8((int8_t)-39) != (int8_t)-39) {
        return 17;
    }
    if (sci_abi_call_host_cast_arg_u8(UINT8_C(0xd1)) != UINT8_C(0xd1)) {
        return 18;
    }
    if (sci_abi_call_host_cast_arg_i16((int16_t)-4321) != (int16_t)-4321) {
        return 19;
    }
    if (sci_abi_call_host_cast_arg_u16(UINT16_C(0xd00d)) != UINT16_C(0xd00d)) {
        return 20;
    }
    if (sci_abi_call_host_cast_arg_i32(INT32_C(-7654321)) != INT32_C(-7654321)) {
        return 21;
    }
    if (sci_abi_call_host_cast_arg_u32(UINT32_C(0xd00d1234)) != UINT32_C(0xd00d1234)) {
        return 22;
    }
    if (sci_abi_call_host_cast_arg_i64(-INT64_C(7654321012345)) != -INT64_C(7654321012345)) {
        return 23;
    }
    if (sci_abi_call_host_cast_arg_u64(UINT64_C(0xd00dba9876543210)) != UINT64_C(0xd00dba9876543210)) {
        return 24;
    }
    if (sci_abi_call_host_cast_return_i8((int8_t)-61) != (int8_t)-61) {
        return 25;
    }
    if (sci_abi_call_host_cast_return_u8(UINT8_C(0xe3)) != UINT8_C(0xe3)) {
        return 26;
    }
    if (sci_abi_call_host_cast_return_i16((int16_t)-6789) != (int16_t)-6789) {
        return 27;
    }
    if (sci_abi_call_host_cast_return_u16(UINT16_C(0xe11e)) != UINT16_C(0xe11e)) {
        return 28;
    }
    if (sci_abi_call_host_cast_return_i32(INT32_C(-1357911)) != INT32_C(-1357911)) {
        return 29;
    }
    if (sci_abi_call_host_cast_return_u32(UINT32_C(0xe11e1234)) != UINT32_C(0xe11e1234)) {
        return 30;
    }
    if (sci_abi_call_host_cast_return_i64(-INT64_C(1357911012345)) != -INT64_C(1357911012345)) {
        return 31;
    }
    if (sci_abi_call_host_cast_return_u64(UINT64_C(0xe11eba9876543210)) != UINT64_C(0xe11eba9876543210)) {
        return 32;
    }
    return 0;
}
