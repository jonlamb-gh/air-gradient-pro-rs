use crate::{
    archive_util::{self, BootSlotExt},
    interruptor::Interruptor,
    opts::ExtractArchive,
};
use anyhow::{bail, Result};
use bootloader_support::BootSlot;
use elf::{endian::LittleEndian, ElfBytes};
use std::fs;

// TODO - add tests in CI that show the bins output from this match what
// arm-none-eabi-objcopy -O binary
// produces
//
// this could use some cleanup
pub async fn extract_archive(cmd: ExtractArchive, _intr: Interruptor) -> Result<()> {
    println!(
        "Extracting '{}' to '{}'",
        cmd.agp_images_cpio_file.display(),
        cmd.output_dir.display()
    );

    if !cmd.agp_images_cpio_file.exists() {
        bail!(
            "Image archive '{}' does not exist",
            cmd.agp_images_cpio_file.display()
        );
    }

    if !cmd.output_dir.exists() {
        fs::create_dir_all(&cmd.output_dir)?;
    }

    let (elf_slot0_data, elf_slot1_data) =
        archive_util::extract_elf_files_from_archive(&cmd.agp_images_cpio_file)?;

    let elf_slot0 = ElfBytes::<LittleEndian>::minimal_parse(&elf_slot0_data)?;
    let elf_slot1 = ElfBytes::<LittleEndian>::minimal_parse(&elf_slot1_data)?;

    let elf_path = cmd.output_dir.join(BootSlot::Slot0.elf_file_name_and_ext());
    println!("Writing ELF '{}'", elf_path.display());
    fs::write(elf_path, &elf_slot0_data)?;

    let bin_slot0_data = archive_util::elf2bin(BootSlot::Slot0, &elf_slot0)?;
    let bin_path = cmd.output_dir.join(BootSlot::Slot0.bin_file_name_and_ext());
    println!("Writing bin '{}'", bin_path.display());
    fs::write(bin_path, bin_slot0_data)?;

    let elf_path = cmd.output_dir.join(BootSlot::Slot1.elf_file_name_and_ext());
    println!("Writing ELF '{}'", elf_path.display());
    fs::write(elf_path, &elf_slot1_data)?;

    let bin_slot1_data = archive_util::elf2bin(BootSlot::Slot1, &elf_slot1)?;
    let bin_path = cmd.output_dir.join(BootSlot::Slot1.bin_file_name_and_ext());
    println!("Writing bin '{}'", bin_path.display());
    fs::write(bin_path, bin_slot1_data)?;

    Ok(())
}
