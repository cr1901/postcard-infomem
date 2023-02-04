SECTIONS {
    /* Required, otherwise won't be treated as valid Win32 app.. */
    . = ALIGN(__section_alignment__);
    .info : {
      _sinfo = .;
      KEEP(*(.info))
      _einfo = .;
    }
} INSERT AFTER .text
