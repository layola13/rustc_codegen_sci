#include <stdint.h>

struct PairU64 {
    uint64_t left;
    uint64_t right;
};

extern uint64_t sci_abi_pair_arg(struct PairU64 value);
extern uint64_t sci_abi_call_host_pair_arg(uint64_t left, uint64_t right);
extern struct PairU64 sci_abi_pair_return(uint64_t left, uint64_t right);

uint64_t sci_host_pair_arg(struct PairU64 value) {
    return value.left + value.right;
}

int main(void) {
    if (sci_abi_pair_arg((struct PairU64){ .left = UINT64_C(11), .right = UINT64_C(31) }) != UINT64_C(42)) {
        return 1;
    }
    if (sci_abi_call_host_pair_arg(UINT64_C(13), UINT64_C(29)) != UINT64_C(42)) {
        return 2;
    }
    struct PairU64 returned = sci_abi_pair_return(UINT64_C(17), UINT64_C(25));
    if (returned.left != UINT64_C(17) || returned.right != UINT64_C(25)) {
        return 3;
    }
    return 0;
}
