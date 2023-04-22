use crate::{
    archive_util::{self, BootSlotExt},
    device_util::{self, DeviceInfo},
    interruptor::Interruptor,
    opts::DeviceUpdate,
};
use anyhow::{bail, Result};
use bootloader_support::BootSlot;
use elf::{endian::LittleEndian, ElfBytes};
use std::{fs, net};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tracing::debug;
use wire_protocols::device::{Command, MemoryRegion};

pub async fn update(cmd: DeviceUpdate, _intr: Interruptor) -> Result<()> {
    if !cmd.agp_images_cpio_file.exists() {
        bail!(
            "Image archive '{}' does not exist",
            cmd.agp_images_cpio_file.display()
        );
    }

    println!(
        "Updating system from {}:{} with image archive '{}'",
        cmd.common.address,
        cmd.common.port,
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

    let s = net::TcpStream::connect((cmd.common.address.as_str(), cmd.common.port))?;
    s.set_nonblocking(true)?;
    let mut stream = TcpStream::from_std(s)?;

    debug!("Requesting device info");
    device_util::write_command(Command::Info, &mut stream).await?;
    let _status = device_util::read_status(&mut stream).await?;

    let mut buf_stream = BufReader::new(stream);
    let mut info_str = String::new();
    let _info_len = buf_stream.read_line(&mut info_str).await?;
    let info = DeviceInfo::from_json(&info_str)?;
    let stream = buf_stream.into_inner();
    if cmd.common.format.is_text() {
        println!("{info:#?}");
    }

    let s = stream.into_std()?;
    s.shutdown(net::Shutdown::Both)?;
    drop(s);

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Re-connect after info command
    let s = net::TcpStream::connect((cmd.common.address.as_str(), cmd.common.port))?;
    s.set_nonblocking(true)?;
    let mut stream = TcpStream::from_std(s)?;

    let current_boot_slot_from_info = BootSlot::Slot0;
    let boot_slot_to_update = current_boot_slot_from_info.other();

    let elf_to_use = match boot_slot_to_update {
        BootSlot::Slot0 => elf_slot0,
        BootSlot::Slot1 => elf_slot1,
    };

    let bin_data = archive_util::elf2bin(boot_slot_to_update, &elf_to_use)?;
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

    if cmd.common.format.is_text() {
        println!("Erasing sectors for boot slot {boot_slot_to_update}");
    }
    let mem_region_to_erase =
        MemoryRegion::new_unchecked(boot_slot_to_update.address(), boot_slot_to_update.size());
    device_util::write_command(Command::EraseMemory, &mut stream).await?;
    stream.write_all(&mem_region_to_erase.to_le_bytes()).await?;
    let status = device_util::read_status(&mut stream).await?;
    if cmd.common.format.is_text() {
        println!("Erase status: {status}");
    }

    // TODO
    // do netowrk protocol to write to slot
    // ...

    Ok(())
}
