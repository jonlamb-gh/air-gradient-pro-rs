use crate::{
    archive_util::{self, BootSlotExt},
    interruptor::Interruptor,
    opts::DeviceUpdate,
};
use anyhow::{bail, Result};
use bootloader_support::BootSlot;
use elf::{endian::LittleEndian, ElfBytes};
use std::fs;
use tracing::debug;

pub async fn update(cmd: DeviceUpdate, _intr: Interruptor) -> Result<()> {
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

    let (elf_slot0_data, elf_slot1_data) =
        archive_util::extract_elf_files_from_archive(&cmd.agp_images_cpio_file)?;

    if let Some(c) = cmd.cache_dir.as_ref() {
        let elf_path = c.join(BootSlot::Slot0.elf_file_name_and_ext());
        debug!("Writing ELF '{}'", elf_path.display());
        fs::write(elf_path, &elf_slot0_data)?;

        let elf_path = c.join(BootSlot::Slot1.elf_file_name_and_ext());
        debug!("Writing ELF '{}'", elf_path.display());
        fs::write(elf_path, &elf_slot1_data)?;
    }

    let elf_slot0 = ElfBytes::<LittleEndian>::minimal_parse(&elf_slot0_data)?;
    let elf_slot1 = ElfBytes::<LittleEndian>::minimal_parse(&elf_slot1_data)?;

    // At this point the archive and image files look ok

    // TODO - do network protocol to get info
    // includes current boot slot
    let current_boot_slot_from_info = BootSlot::Slot0;
    let boot_slot_to_update = current_boot_slot_from_info.other();

    let elf_to_use = match boot_slot_to_update {
        BootSlot::Slot0 => elf_slot0,
        BootSlot::Slot1 => elf_slot1,
    };

    let bin_data = archive_util::elf2bin(boot_slot_to_update, &elf_to_use)?;
    // TODO
    if bin_data.len() > boot_slot_to_update.size() as usize {
        bail!(
            "Firmware must fit into boot slot size {}",
            boot_slot_to_update.size()
        );
    }

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
