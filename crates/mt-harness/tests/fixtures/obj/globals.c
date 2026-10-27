/* Fixture for eqv_04_globals_are_found_in_data_and_bss (plan 03-04, D-10).
 *
 * Built with:
 *   arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16 \
 *       -Os -c globals.c -o globals.o
 *
 * Writable globals (.data and .bss): counter, table, zeroed, hidden_bss.
 * Not writable: ro (.rodata). tag and use() are code or read-only data.
 */
int counter = 5;
static unsigned char table[7] = {1, 2, 3, 4, 5, 6, 7};
int zeroed[3];
static int hidden_bss;
const int ro = 9;

int use(void)
{
    table[1]++;
    hidden_bss += table[1];
    return hidden_bss + ro + zeroed[0];
}
