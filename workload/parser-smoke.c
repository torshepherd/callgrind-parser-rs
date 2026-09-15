/* Small deterministic producer smoke workload. Not part of Cargo unit tests. */
static volatile unsigned result;

static unsigned work(unsigned value) {
    if (value < 2) return value + 1;
    return work(value - 1) + work(value - 2);
}

int main(void) {
    for (unsigned i = 0; i < 12; ++i) {
        result += work(i);
        if ((i & 1) == 0) result += i;
    }
    return 0;
}
