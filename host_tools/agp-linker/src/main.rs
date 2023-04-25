use cpio::{write_cpio, NewcBuilder};
use std::{
    env, fs, io,
    path::Path,
    process::{self, Command, ExitStatus},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const EXIT_CODE_FAILURE: i32 = 1;

const LINKER: &str = "flip-link";

const NO_ARCHIVE_ENV_VAR: &str = "AGP_LINKER_NO_ARCHIVE";

const MEMORY_FILE_NAME: &str = "memory.x";
const FLASH_SLOT0_MEMORY_FILE_NAME: &str = "agp_memory_slot_0.x";
const FLASH_SLOT1_MEMORY_FILE_NAME: &str = "agp_memory_slot_1.x";

const ELF_SLOT0: &str = "agp0.elf";
const ELF_SLOT1: &str = "agp1.elf";

const CPIO_ARCHIVE: &str = "agp_images.cpio";

fn main() -> Result<()> {
    notmain().map(|code| process::exit(code))
}

fn notmain() -> Result<i32> {
    env_logger::init();

    // NOTE `skip` the name/path of the binary (first argument)
    let args = env::args().skip(1).collect::<Vec<_>>();

    let exit_status = link_normally(&args)?;
    if !exit_status.success() {
        eprintln!(
            "\nagp-linker: the native linker failed to link the program normally; \
                 please check your project configuration and linker scripts"
        );
        return Ok(exit_status.code().unwrap_or(EXIT_CODE_FAILURE));
    }

    if env::var_os(NO_ARCHIVE_ENV_VAR).is_none() {
        let current_dir = env::current_dir()?;
        let target_dir = current_dir.join("target");
        let _output_elf_path = get_output_path(&args)?;

        let memory_x_path = current_dir.join(MEMORY_FILE_NAME);
        let memory_x_content = fs::read_to_string(&memory_x_path)?;

        let agp_memory_slot0_path = current_dir.join(FLASH_SLOT0_MEMORY_FILE_NAME);
        if !agp_memory_slot0_path.exists() {
            eprintln!(
                "\nagp-linker: AGP memory for slot 0 is missing; Create file '{FLASH_SLOT0_MEMORY_FILE_NAME}'."
            );
            return Ok(EXIT_CODE_FAILURE);
        }

        let agp_memory_slot1_path = current_dir.join(FLASH_SLOT1_MEMORY_FILE_NAME);
        if !agp_memory_slot1_path.exists() {
            eprintln!(
                "\nagp-linker: AGP memory for slot 1 is missing; Create file '{FLASH_SLOT1_MEMORY_FILE_NAME}'."
            );
            return Ok(EXIT_CODE_FAILURE);
        }

        // Link with slot 0 memory config
        let agp_memory_slot0_content = fs::read_to_string(&agp_memory_slot0_path)?;
        fs::write(&memory_x_path, agp_memory_slot0_content)?;
        let agp_slot0_elf = target_dir.join(ELF_SLOT0);
        let slot0_linker_args = replace_output_path(&args, &agp_slot0_elf)?;

        let exit_status = link_normally(&slot0_linker_args)?;
        if !exit_status.success() {
            return Ok(exit_status.code().unwrap_or(EXIT_CODE_FAILURE));
        }

        // Link with slot 1 memory config
        let agp_memory_slot1_content = fs::read_to_string(&agp_memory_slot1_path)?;
        fs::write(&memory_x_path, agp_memory_slot1_content)?;
        let agp_slot1_elf = target_dir.join(ELF_SLOT1);
        let slot1_linker_args = replace_output_path(&args, &agp_slot1_elf)?;

        let exit_status = link_normally(&slot1_linker_args)?;
        if !exit_status.success() {
            return Ok(exit_status.code().unwrap_or(EXIT_CODE_FAILURE));
        }

        // Restore user's memory.x
        fs::write(memory_x_path, memory_x_content)?;

        // Create a CPIO archive with the two ELF files
        let cpio_archive_path = target_dir.join(CPIO_ARCHIVE);
        let mut cpio_archive_file = fs::File::create(cpio_archive_path)?;
        let agp_slot0_elf = fs::File::open(agp_slot0_elf)?;
        let agp_slot1_elf = fs::File::open(agp_slot1_elf)?;
        let cpio_inputs = vec![
            (NewcBuilder::new(ELF_SLOT0), agp_slot0_elf),
            (NewcBuilder::new(ELF_SLOT1), agp_slot1_elf),
        ];
        let _ = write_cpio(cpio_inputs.into_iter(), &mut cpio_archive_file)?;
    }

    Ok(0)
}

/// Normal linking with just the arguments the user provides
fn link_normally(args: &[String]) -> io::Result<ExitStatus> {
    let mut c = Command::new(LINKER);
    c.args(args);
    log::trace!("{:?}", c);

    c.status()
}

/// Get `output_path`, specified by `-o`
fn get_output_path(args: &[String]) -> Result<&String> {
    args.windows(2)
        .find_map(|x| (x[0] == "-o").then(|| &x[1]))
        .ok_or_else(|| "(BUG?) `-o` flag not found".into())
}

/// Replace the `output_path`, specified by `-o`
fn replace_output_path<P: AsRef<Path>>(args: &[String], output_path: P) -> Result<Vec<String>> {
    let mut new_args = args.to_vec();
    let output_idx = new_args
        .iter()
        .position(|x| x == "-o")
        .ok_or_else(|| "(BUG?) `-o` flag not found".to_string())?;
    new_args[output_idx + 1] = output_path.as_ref().display().to_string();
    Ok(new_args)
}
