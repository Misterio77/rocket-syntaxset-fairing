//! Easily manage a syntect syntaxset through Rocket.
use normpath::PathExt;
use rocket::{
    error,
    fairing::{self, Fairing, Info, Kind},
    info, info_,
    outcome::IntoOutcome,
    request::{self, FromRequest, Request},
    Build, Orbit, Rocket,
};
use std::ops::Deref;
use std::path::PathBuf;
use syntect::parsing::SyntaxSet;

pub struct Syntaxes {
    inner: SyntaxSet,
    path: PathBuf,
}

impl Deref for Syntaxes {
    type Target = SyntaxSet;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Syntaxes {
    pub fn fairing() -> impl Fairing {
        SyntaxesFairing
    }
}

struct SyntaxesFairing;

#[rocket::async_trait]
impl Fairing for SyntaxesFairing {
    fn info(&self) -> Info {
        Info {
            kind: Kind::Ignite | Kind::Liftoff,
            name: "Syntaxes",
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        use rocket::figment::value::magic::RelativePathBuf;

        let configured_path = rocket
            .figment()
            .extract_inner::<RelativePathBuf>("syntaxes_path")
            .map(|path| path.relative());

        let relative_path = match configured_path {
            Ok(path) => path,
            Err(e) if e.missing() => "syntaxes".into(),
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        let path = match relative_path.normalize() {
            Ok(path) => path.into_path_buf(),
            Err(e) => {
                error!(
                    "Invalid syntaxes path '{}': {}.",
                    relative_path.display(),
                    e
                );
                return Err(rocket);
            }
        };

        let syntaxes = match SyntaxSet::load_from_folder(&path) {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "Couldn't load syntaxes from '{}': {}",
                    relative_path.display(),
                    e
                );
                return Err(rocket);
            }
        };

        Ok(rocket.manage(Syntaxes {
            inner: syntaxes,
            path,
        }))
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        use rocket::{figment::Source, log::PaintExt, yansi::Paint};

        let state = rocket
            .state::<Syntaxes>()
            .expect("Syntaxes registered in on_ignite");

        info!("{}{}:", Paint::emoji("📐 "), Paint::magenta("Syntaxes"));
        info_!("syntax path: {}", Paint::white(Source::from(&*state.path)));
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Syntaxes {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, ()> {
        req.rocket().state::<Syntaxes>().or_forward(())
    }
}
