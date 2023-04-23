use crate::{
    archive_util::{self, BootSlotExt},
    device_util::{self, DeviceInfo},
    interruptor::Interruptor,
    opts::DeviceUpdate,
};
use anyhow::{anyhow, bail, Result};
use bootloader_support::BootSlot;
use elf::{endian::LittleEndian, ElfBytes};
use std::{fs, io::Write, net};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
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
    s.set_nodelay(true)?;
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

    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    // Re-connect after info command
    let s = net::TcpStream::connect((cmd.common.address.as_str(), cmd.common.port))?;
    s.set_nonblocking(true)?;
    s.set_nodelay(true)?;
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
        fs::write(bin_path, &bin_data)?;
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

    if cmd.common.format.is_text() {
        println!(
            "Wrting bin to boot slot {boot_slot_to_update}, {} bytes",
            bin_data.len()
        );
    }
    let mut write_address = boot_slot_to_update.address();
    let num_chunks = divide_round_up(bin_data.len(), MemoryRegion::MAX_CHUCK_SIZE);
    for (chunk_idx, chunk) in bin_data.chunks(MemoryRegion::MAX_CHUCK_SIZE).enumerate() {
        debug!(
            "Sending bin chunk address=0x{:X}, len=0x{:X}, {} of {}",
            write_address,
            chunk.len(),
            chunk_idx + 1,
            num_chunks,
        );

        let mem_region_to_write = MemoryRegion::new_unchecked(write_address, chunk.len() as u32);
        mem_region_to_write
            .check_length()
            .map_err(|sc| anyhow!("Memory region to write is invalid. {sc}"))?;
        device_util::write_command(Command::WriteMemory, &mut stream).await?;
        stream.write_all(&mem_region_to_write.to_le_bytes()).await?;
        stream.write_all(chunk).await?;
        let _status = device_util::read_status(&mut stream).await?;

        write_address += mem_region_to_write.length;
    }

    if cmd.common.format.is_text() {
        println!("Verifying image currently in {boot_slot_to_update}");
    }

    let mut readback_file = match cmd.cache_dir.as_ref() {
        None => None,
        Some(c) => {
            let bin_path = c.join("mem_readback.bin");
            Some(fs::File::create(bin_path)?)
        }
    };

    let mut read_address = boot_slot_to_update.address();
    for (chunk_idx, chunk) in bin_data.chunks(MemoryRegion::MAX_CHUCK_SIZE).enumerate() {
        debug!(
            "Read bin chunk address=0x{:X}, len=0x{:X}, {} of {}",
            read_address,
            chunk.len(),
            chunk_idx + 1,
            num_chunks,
        );

        let mem_region_to_read = MemoryRegion::new_unchecked(read_address, chunk.len() as u32);
        mem_region_to_read
            .check_length()
            .map_err(|sc| anyhow!("Memory region to read is invalid. {sc}"))?;
        device_util::write_command(Command::ReadMemory, &mut stream).await?;
        stream.write_all(&mem_region_to_read.to_le_bytes()).await?;
        let _status = device_util::read_status(&mut stream).await?;
        let mut bin_data_read_back_from_dev = vec![0_u8; chunk.len()];
        let num_bytes_read = stream.read_exact(&mut bin_data_read_back_from_dev).await?;
        if num_bytes_read != chunk.len() {
            bail!(
                "Read back {num_bytes_read} but was expecting {}",
                chunk.len()
            );
        }

        if let Some(f) = readback_file.as_mut() {
            f.write_all(&bin_data_read_back_from_dev)?;
        }

        if bin_data_read_back_from_dev.as_slice() != chunk {
            bail!(
                "Chunk {} does not match what we sent, aborting",
                chunk_idx + 1
            );
        }

        read_address += mem_region_to_read.length;
    }

    if cmd.common.format.is_text() {
        println!("Update complete, issue reboot command");
    }

    device_util::write_command(Command::CompleteAndReboot, &mut stream).await?;

    Ok(())
}

// a/b
fn divide_round_up(a: usize, b: usize) -> usize {
    (a + (b - 1)) / b
}
