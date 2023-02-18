# This is a wrapper script for progammer applications demonstrating how to
# write information memory to microcontrollers.
#
# You can use a file called .env in the same directory as this Justfile to
# tailor the programmer invocation. The commands will be written out when
# this Justfile is run.
#
# At some point I'll likely write a Rust wrapper to be more flexible.

set dotenv-load
set windows-powershell

avrdude_part := env_var_or_default('AVRDUDE_PART', 'atmega328p')
avrdude_driver := env_var_or_default('AVRDUDE_DRIVER', 'arduino')
avrdude_port := env_var_or_default('AVRDUDE_PORT', 'COM1')
avrdude_baud := env_var_or_default('AVRDUDE_BAUD', '57600')

elf2uf2_mount := env_var_or_default('ELF2UF2_MOUNT', '.')

mspdebug_driver := env_var_or_default('MSPDEBUG_DRIVER', 'tilib')

avrdude FILE:
    avrdude -p{{avrdude_part}} -c{{avrdude_driver}} -P{{avrdude_port}} -b{{avrdude_baud}} -Uflash:w:{{FILE}}:e
    avr-objcopy -j.eeprom -Obinary {{FILE}} eeprom.bin
    avrdude -p{{avrdude_part}} -c{{avrdude_driver}} -P{{avrdude_port}} -b{{avrdude_baud}} -D -Ueeprom:w:eeprom.bin
    rm eeprom.bin

# Don't use -d... don't assume we're talking to a PICO...
elf2uf2-rs FILE:
    elf2uf2-rs {{FILE}} {{elf2uf2_mount}}/out.uf2

mspdebug FILE:
    mspdebug {{mspdebug_driver}} 'erase segrange 0x1000 192 64' 'prog {{FILE}}'
