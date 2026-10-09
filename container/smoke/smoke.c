/* C caller for the Rust smoke crate (TOOL-04).
 *
 * Compiled by arm-none-eabi-gcc in the container workflow:
 *   arm-none-eabi-gcc -mcpu=cortex-m4 -mthumb -mfloat-abi=hard \
 *     -mfpu=fpv4-sp-d16 -Os -c smoke.c
 */
#include <stdint.h>

uint32_t mt_smoke_add(uint32_t a, uint32_t b);

uint32_t mt_smoke_caller(uint32_t x)
{
    return mt_smoke_add(x, 1u);
}
