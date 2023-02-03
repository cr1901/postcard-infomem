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

ASSERT((_einfo - _sinfo) < 255, "
ERROR(memory.x): Information memory is greater than 254 bytes and overwrites MSP430Gx2xx calibration data");


