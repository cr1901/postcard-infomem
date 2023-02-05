MEMORY
{
  RAM : ORIGIN = 0x0200, LENGTH = 0x0200
  INFOMEM : ORIGIN = 0x1000, LENGTH = 0x100
  ROM : ORIGIN = 0xC000, LENGTH = 0x3FDE
  VECTORS : ORIGIN = 0xFFE0, LENGTH = 0x20
}

SECTIONS {
    .info : {
      _sinfo = .;
      KEEP(*(.info))
      _einfo = .;
    } > INFOMEM
}

/* This is a precaution. If you have a way to save the calibration data before
it's been erased, all 256-2 bytes of information memory can be used. */
ASSERT((_einfo - _sinfo) <= 192, "
ERROR(memory.x): Information memory is greater than 192 bytes long. Erasing flash
to write the information memory would also erase (and possibly overwrite) MSP430Gx2xx
calibration data.");


