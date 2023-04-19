use crate::{interruptor::Interruptor, opts::DeviceUpdate};
use anyhow::{anyhow, bail, Result};
use bootloader_support::BootSlot;
use cpio::NewcReader;
use elf::{
    abi,
    endian::LittleEndian,
    file::{Class, FileHeader},
    segment::ProgramHeader,
    ElfBytes,
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io,
    path::PathBuf,
};
use tracing::debug;

pub async fn update(cmd: DeviceUpdate, intr: Interruptor) -> Result<()> {
    if !cmd.agp_images_cpio_file.exists() {
        bail!(
            "Image archive '{}' does not exist",
            cmd.agp_images_cpio_file.display()
        );
    }

    println!(
        "Updating system with image archive '{}'",
        cmd.agp_images_cpio_file.display()
    );

    if let Some(c) = cmd.cache_dir.as_ref() {
        debug!("Using image cache dir '{}'", c.display());
        fs::create_dir_all(c)?;
    }

    let archive_file = File::open(&cmd.agp_images_cpio_file)?;
    let mut archive_reader = NewcReader::new(&archive_file)?;

    let elf_slot0_data = load_elf_data(BootSlot::Slot0, &mut archive_reader)?;
    if let Some(c) = cmd.cache_dir.as_ref() {
        let elf_path = c.join(BootSlot::Slot0.elf_file_name_and_ext());
        debug!("Writing ELF '{}'", elf_path.display());
        fs::write(elf_path, &elf_slot0_data)?;
    }
    let elf_slot0 = ElfBytes::<LittleEndian>::minimal_parse(&elf_slot0_data)?;
    sanity_check_elf(BootSlot::Slot0, &elf_slot0.ehdr)?;

    let mut archive_reader = NewcReader::new(archive_reader.finish()?)?;

    let elf_slot1_data = load_elf_data(BootSlot::Slot1, &mut archive_reader)?;
    if let Some(c) = cmd.cache_dir.as_ref() {
        let elf_path = c.join(BootSlot::Slot1.elf_file_name_and_ext());
        debug!("Writing ELF '{}'", elf_path.display());
        fs::write(elf_path, &elf_slot1_data)?;
    }
    let elf_slot1 = ElfBytes::<LittleEndian>::minimal_parse(&elf_slot1_data)?;
    sanity_check_elf(BootSlot::Slot1, &elf_slot1.ehdr)?;

    // At this point the archive and image files look ok

    // TODO - do network protocol to get info
    // includes current boot slot
    let current_boot_slot_from_info = BootSlot::Slot0;
    let boot_slot_to_update = current_boot_slot_from_info.other();

    let elf_to_use = match boot_slot_to_update {
        BootSlot::Slot0 => elf_slot0,
        BootSlot::Slot1 => elf_slot1,
    };

    let bin_data = elf2bin(boot_slot_to_update, &elf_to_use)?;
    if let Some(c) = cmd.cache_dir.as_ref() {
        let bin_path = c.join(BootSlot::Slot1.bin_file_name_and_ext());
        debug!("Writing bin '{}'", bin_path.display());
        fs::write(bin_path, bin_data)?;
    }

    // TODO
    // do netowrk protocol to write to slot
    // ...

    Ok(())
}

const ELF_SLOT0_FILE_NAME: &str = "agp0";
const ELF_SLOT1_FILE_NAME: &str = "agp1";

trait BootSlotExt {
    fn file_name(&self) -> &'static str;

    fn elf_file_name_and_ext(&self) -> PathBuf {
        let mut f = PathBuf::from(self.file_name());
        f.set_extension("elf");
        f
    }

    fn bin_file_name_and_ext(&self) -> PathBuf {
        let mut f = PathBuf::from(self.file_name());
        f.set_extension("bin");
        f
    }
}

impl BootSlotExt for BootSlot {
    fn file_name(&self) -> &'static str {
        match self {
            BootSlot::Slot0 => ELF_SLOT0_FILE_NAME,
            BootSlot::Slot1 => ELF_SLOT1_FILE_NAME,
        }
    }
}

fn load_elf_data<R: io::Read>(slot: BootSlot, reader: &mut NewcReader<R>) -> Result<Vec<u8>> {
    let entry = reader.entry();
    debug!(
        "Found entry '{}' with size {} in archive",
        entry.name(),
        entry.file_size()
    );
    if entry.name() != slot.elf_file_name_and_ext().to_str().unwrap() {
        bail!(
            "Bad archive, first entry should be {}, found '{}'",
            slot.elf_file_name_and_ext().to_str().unwrap(),
            entry.name()
        );
    }
    let mut data = Vec::new();
    io::copy(reader, &mut data)?;
    Ok(data)
}

fn sanity_check_elf(slot: BootSlot, ehdr: &FileHeader<LittleEndian>) -> Result<()> {
    if ehdr.class != Class::ELF32 {
        bail!("Bad class");
    }
    if ehdr.e_machine != abi::EM_ARM {
        bail!("Bad e_machine");
    }
    if !slot.contains(ehdr.e_entry as u32) {
        bail!("Bad e_entry 0x{:X}", ehdr.e_entry);
    }
    Ok(())
}

const SH_VECTOR_TABLE: &str = ".vector_table";
const SH_TEXT: &str = ".text";
const SH_RODATA: &str = ".rodata";
const SH_DATA: &str = ".data";

// TODO
// - better error handling/reporting
// - maybe add a dry-run or w/e and generate the bin
//   in CI, sanity check matches what arm-none-eabi-objcopy -O binary would do
fn elf2bin(slot: BootSlot, elf: &ElfBytes<LittleEndian>) -> Result<Vec<u8>> {
    debug!("Converting boot slot {slot} ELF to bin");

    let vector_table_sh = elf
        .section_header_by_name(SH_VECTOR_TABLE)?
        .ok_or_else(|| anyhow!("Missing {SH_VECTOR_TABLE} section header"))?;
    let text_sh = elf
        .section_header_by_name(SH_TEXT)?
        .ok_or_else(|| anyhow!("Missing {SH_TEXT} section header"))?;
    let rodata_sh = elf
        .section_header_by_name(SH_RODATA)?
        .ok_or_else(|| anyhow!("Missing {SH_RODATA} section header"))?;
    let data_sh = elf
        .section_header_by_name(SH_DATA)?
        .ok_or_else(|| anyhow!("Missing {SH_DATA} section header"))?;

    if vector_table_sh.sh_addr != slot.address() as u64 {
        bail!(
            "Bad {SH_VECTOR_TABLE} address 0x{:X}",
            vector_table_sh.sh_addr
        );
    }
    if !slot.contains(text_sh.sh_addr as u32) {
        bail!("Bad {SH_TEXT} address 0x{:X}", text_sh.sh_addr);
    }
    if !slot.contains(rodata_sh.sh_addr as u32) {
        bail!("Bad {SH_RODATA} address 0x{:X}", rodata_sh.sh_addr);
    }

    let offset_to_address_alignments: BTreeMap<u64, u64> =
        [&vector_table_sh, &text_sh, &rodata_sh, &data_sh]
            .iter()
            .map(|sh| (sh.sh_offset, sh.sh_addralign))
            .collect();

    let program_headers: Vec<ProgramHeader> = elf
        .segments()
        .ok_or_else(|| anyhow!("Missing program headers"))?
        .into_iter()
        .collect();

    let vector_table_ph = program_headers
        .iter()
        .find(|ph| ph.p_offset == vector_table_sh.sh_offset)
        .ok_or_else(|| anyhow!("Missing {SH_VECTOR_TABLE} program header"))?;
    if vector_table_ph.p_flags != abi::PF_R {
        bail!("Bad {SH_VECTOR_TABLE} program header flags");
    }

    let text_ph = program_headers
        .iter()
        .find(|ph| ph.p_offset == text_sh.sh_offset)
        .ok_or_else(|| anyhow!("Missing {SH_TEXT} program header"))?;
    if text_ph.p_flags != abi::PF_R | abi::PF_X {
        bail!("Bad {SH_TEXT} program header flags");
    }

    let rodata_ph = program_headers
        .iter()
        .find(|ph| ph.p_offset == rodata_sh.sh_offset)
        .ok_or_else(|| anyhow!("Missing {SH_RODATA} program header"))?;
    if rodata_ph.p_flags != abi::PF_R {
        bail!("Bad {SH_RODATA} program header flags");
    }

    let data_ph = program_headers
        .iter()
        .find(|ph| ph.p_offset == data_sh.sh_offset)
        .ok_or_else(|| anyhow!("Missing {SH_DATA} program header"))?;
    if data_ph.p_flags != abi::PF_R | abi::PF_W {
        bail!("Bad {SH_DATA} program header flags");
    }

    let mut combined_segment_data = Vec::new();
    let mut bin_offset = 0_u64;
    let segments_to_write = vec![vector_table_ph, text_ph, rodata_ph, data_ph];
    for ph in segments_to_write.into_iter() {
        let address_alignment = *offset_to_address_alignments
            .get(&ph.p_offset)
            .ok_or_else(|| anyhow!("Missing address alignment entry"))?;
        let bin_offset_ptr = bin_offset as u32 as *const u32;
        let bin_align_offset_in_words = bin_offset_ptr.align_offset(address_alignment as usize);

        debug!(
            "Adding segment to bin at 0x{:X} (+ {} align words), vaddr 0x{:X}, paddr 0x{:X}, memsz 0x{:X} ({})",
            bin_offset, bin_align_offset_in_words, ph.p_vaddr, ph.p_paddr, ph.p_memsz, ph.p_memsz
        );

        if ph.p_type != abi::PT_LOAD {
            bail!("Bad program header type");
        }

        if ph.p_filesz != ph.p_memsz {
            bail!("FLASH segment sizes should match what's in the file");
        }

        if !slot.contains(ph.p_paddr as u32) {
            bail!("Bad p_paddr 0x{:X}", ph.p_paddr);
        }

        let seg_data = elf.segment_data(ph)?;

        // Start with padding zero bytes to align the address
        for _ in 0..bin_align_offset_in_words {
            combined_segment_data.extend_from_slice(0_u32.to_le_bytes().as_slice());
            bin_offset += 4;
        }

        combined_segment_data.extend_from_slice(seg_data);

        bin_offset += ph.p_memsz;
    }

    Ok(combined_segment_data)
}
