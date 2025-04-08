
ENTRY(_start)

PHDRS {
  text    PT_LOAD FLAGS(5);  /* R | X */
  data    PT_LOAD FLAGS(6);  /* R | W */
  bss     PT_LOAD FLAGS(6);  /* R | W (no file data, but still needs memory) */
}

SECTIONS {
  . = 0x500000;

  .text : ALIGN(0x1000) {
    *(.text)
  } :text

  .data : ALIGN(0x1000) {
    *(.data)
  } :data

  .bss : ALIGN(0x1000) {
    *(.bss)
    *(COMMON)
  } :bss
}
