#include <stdint.h>

extern int32_t sci_add_i32(int32_t a, int32_t b);
extern int32_t sci_gt_i32(int32_t a, int32_t b);
extern int32_t sci_max_i32(int32_t a, int32_t b);
extern int32_t sci_tuple_sum_i32(int32_t a, int32_t b);
extern int32_t sci_tuple_copy_sum_i32(int32_t a, int32_t b);
extern int32_t sci_struct_sum_i32(int32_t a, int32_t b);
extern int32_t sci_struct_copy_sum_i32(int32_t a, int32_t b);
extern int32_t sci_empty_struct_local_i32(int32_t a, int32_t b);
extern const int32_t *sci_identity_ptr(const int32_t *value);
extern const int32_t *sci_call_identity_ptr(const int32_t *value);
extern const int32_t *sci_call_host_identity_ptr(const int32_t *value);
extern int32_t sci_ptr_eq(const int32_t *lhs, const int32_t *rhs);
extern int32_t sci_ptr_is_null(const int32_t *value);
extern const int32_t *sci_null_ptr(void);
extern int32_t sci_call_add_i32(int32_t a, int32_t b);
extern int32_t sci_call_host_add_i32(int32_t a, int32_t b);
extern void sci_unit_noop(int32_t a);
extern int32_t sci_call_unit_noop(int32_t a);
extern int32_t sci_call_host_note_i32(int32_t a);
extern int32_t sci_sub_i32(int32_t a, int32_t b);
extern int32_t sci_mul_i32(int32_t a, int32_t b);
extern int64_t sci_mul_i64(int64_t a, int64_t b);
extern uint64_t sci_mul_u64(uint64_t a, uint64_t b);
extern uintptr_t sci_add_usize(uintptr_t a, uintptr_t b);
extern uintptr_t sci_mul_usize(uintptr_t a, uintptr_t b);
extern intptr_t sci_gt_isize(intptr_t a, intptr_t b);
extern int32_t sci_div_i32(int32_t a, int32_t b);
extern int32_t sci_rem_i32(int32_t a, int32_t b);
extern int32_t sci_shl_i32(int32_t a, int32_t b);
extern int32_t sci_shr_i32(int32_t a, int32_t b);
extern int32_t sci_neg_i32(int32_t a);
extern int32_t sci_not_i32(int32_t a);
extern int32_t sci_match_u32(uint32_t a);
extern int32_t sci_match_i32(int32_t a);

int32_t sci_host_add_i32(int32_t a, int32_t b) {
    return a + b;
}

static int32_t host_note_total = 0;

void sci_host_note_i32(int32_t value) {
    host_note_total += value;
}

const int32_t *sci_host_identity_ptr(const int32_t *value) {
    return value;
}

int main(void) {
    static const int32_t ptr_probe = 42;

    if (sci_add_i32(20, 22) != 42) {
        return 1;
    }
    if (sci_gt_i32(7, 3) != 1) {
        return 2;
    }
    if (sci_gt_i32(3, 7) != 0) {
        return 3;
    }
    if (sci_max_i32(7, 3) != 7) {
        return 4;
    }
    if (sci_max_i32(3, 7) != 7) {
        return 5;
    }
    if (sci_tuple_sum_i32(19, 23) != 42) {
        return 32;
    }
    if (sci_tuple_copy_sum_i32(18, 24) != 42) {
        return 34;
    }
    if (sci_struct_sum_i32(17, 25) != 42) {
        return 33;
    }
    if (sci_struct_copy_sum_i32(16, 26) != 42) {
        return 35;
    }
    if (sci_empty_struct_local_i32(15, 27) != 42) {
        return 36;
    }
    if (sci_identity_ptr(&ptr_probe) != &ptr_probe) {
        return 37;
    }
    if (sci_call_identity_ptr(&ptr_probe) != &ptr_probe) {
        return 38;
    }
    if (sci_call_host_identity_ptr(&ptr_probe) != &ptr_probe) {
        return 39;
    }
    if (sci_ptr_eq(&ptr_probe, &ptr_probe) != 1) {
        return 40;
    }
    if (sci_ptr_eq(&ptr_probe, (const int32_t *)0) != 0) {
        return 41;
    }
    if (sci_ptr_is_null((const int32_t *)0) != 1) {
        return 42;
    }
    if (sci_ptr_is_null(&ptr_probe) != 0) {
        return 43;
    }
    if (sci_null_ptr() != (const int32_t *)0) {
        return 44;
    }
    if (sci_call_add_i32(11, 31) != 42) {
        return 6;
    }
    if (sci_call_host_add_i32(20, 22) != 42) {
        return 19;
    }
    sci_unit_noop(5);
    if (sci_call_unit_noop(7) != 42) {
        return 29;
    }
    if (sci_call_host_note_i32(42) != 42) {
        return 30;
    }
    if (host_note_total != 42) {
        return 31;
    }
    if (sci_sub_i32(50, 8) != 42) {
        return 7;
    }
    if (sci_mul_i32(6, 7) != 42) {
        return 8;
    }
    if (sci_mul_i64(3037000499LL, 3LL) != 9111001497LL) {
        return 24;
    }
    if (sci_mul_u64(UINT64_C(4294967296), UINT64_C(10)) != UINT64_C(42949672960)) {
        return 25;
    }
    if (sci_add_usize((uintptr_t)40, (uintptr_t)2) != (uintptr_t)42) {
        return 26;
    }
    if (sci_mul_usize((uintptr_t)7, (uintptr_t)6) != (uintptr_t)42) {
        return 27;
    }
    if (sci_gt_isize((intptr_t)-3, (intptr_t)-7) != (intptr_t)1) {
        return 28;
    }
    if (sci_div_i32(84, 2) != 42) {
        return 9;
    }
    if (sci_rem_i32(85, 43) != 42) {
        return 10;
    }
    if (sci_shl_i32(21, 1) != 42) {
        return 11;
    }
    if (sci_shr_i32(-84, 1) != -42) {
        return 12;
    }
    if (sci_neg_i32(-42) != 42) {
        return 13;
    }
    if (sci_not_i32(-43) != 42) {
        return 14;
    }
    if (sci_match_u32(0) != 7) {
        return 15;
    }
    if (sci_match_u32(1) != 11) {
        return 16;
    }
    if (sci_match_u32(42) != 42) {
        return 17;
    }
    if (sci_match_u32(5) != -1) {
        return 18;
    }
    if (sci_match_i32(-7) != 7) {
        return 20;
    }
    if (sci_match_i32(-1) != 1) {
        return 21;
    }
    if (sci_match_i32(0) != 0) {
        return 22;
    }
    if (sci_match_i32(5) != -42) {
        return 23;
    }
    return 0;
}
