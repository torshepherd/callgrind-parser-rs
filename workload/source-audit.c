/* Native source-audit workload; not part of ordinary Cargo tests.
 * Compile with the Valgrind callgrind/ and include/ directories on -I.
 */
#include "callgrind.h"

static volatile unsigned result;

static unsigned recursive(unsigned value) {
    if (value < 2) return value + 1;
    return recursive(value - 1) + recursive(value - 2);
}

static void phase(void) {
    result += recursive(5);
    CALLGRIND_DUMP_STATS_AT("inside phase, first interval");
    result += recursive(6);
    CALLGRIND_DUMP_STATS_AT("inside phase, second interval");
    result += recursive(7);
}

int main(void) {
    phase();
    return 0;
}
