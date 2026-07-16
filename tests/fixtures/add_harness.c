#include <stdint.h>

extern int32_t sci_add_i32(int32_t a, int32_t b);
extern int32_t sci_gt_i32(int32_t a, int32_t b);
extern int32_t sci_max_i32(int32_t a, int32_t b);
extern int32_t sci_call_add_i32(int32_t a, int32_t b);
extern int32_t sci_sub_i32(int32_t a, int32_t b);
extern int32_t sci_mul_i32(int32_t a, int32_t b);
extern int32_t sci_div_i32(int32_t a, int32_t b);
extern int32_t sci_rem_i32(int32_t a, int32_t b);
extern int32_t sci_shl_i32(int32_t a, int32_t b);
extern int32_t sci_shr_i32(int32_t a, int32_t b);
extern int32_t sci_neg_i32(int32_t a);
extern int32_t sci_not_i32(int32_t a);
extern int32_t sci_match_u32(uint32_t a);

int main(void) {
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
    if (sci_call_add_i32(11, 31) != 42) {
        return 6;
    }
    if (sci_sub_i32(50, 8) != 42) {
        return 7;
    }
    if (sci_mul_i32(6, 7) != 42) {
        return 8;
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
    return 0;
}
