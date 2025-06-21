use color_eyre::eyre::Report;
use invar_pack::Pack;
use invar_pack::instance::version::MinecraftVersion;
use invar_pack::instance::{Instance, Loader};
use invar_pack::settings::Settings;
use invar_repository::persist::PersistedEntity;
use rstest::{fixture, rstest};
use semver::Version;
use tempdir::TempDir;

const TEMPDIR_PREFIX: &str = "invar-repository-test";

#[derive(Debug)]
#[must_use]
pub struct Inputs {
    pub dir: TempDir,
    pub pack: Pack,
}

#[fixture]
fn inputs() -> Inputs {
    // HACK: This dirty closure magic lets us use a single `.unwrap()` down below
    // and `?` everywhere else. Very wacky, but worth it to me.
    (|| -> Result<Inputs, Box<dyn std::error::Error>> {
        color_eyre::install()?;

        let instance = Instance::new(
            MinecraftVersion::from("1.20.1"),
            Loader::Forge,
            Version::parse("47.3.22")?,
        );

        let pack = Pack {
            name: TEMPDIR_PREFIX.into(),
            version: Version::parse("0.1.0")?,
            instance,
            settings: Settings::default(),
            local_components: vec![],
        };

        let dir = TempDir::new(TEMPDIR_PREFIX)?;
        std::env::set_current_dir(dir.path())?;
        pack.write()?;

        Ok(Inputs { dir, pack })
    })()
    .unwrap()
}

#[rstest]
fn persistence(inputs: Inputs) -> Result<(), Report> {
    assert_eq!(Pack::read()?, inputs.pack);
    Ok(())
}
