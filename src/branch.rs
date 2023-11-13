use crate::{
    refs::{RefHandler, Revision},
    workspace::Repository,
};

#[derive(Default, Builder, Debug)]
pub struct Options {
    pub name: Option<String>,
    pub start_point: Option<String>,
}

pub fn branch(options: &Options, repository: &Repository) -> crate::Result<()> {
    if let Some(name) = &options.name {
        let refs = RefHandler::new(repository);

        let start_point = match &options.start_point {
            Some(start_point) => Revision::parse(start_point)?.resolve(repository)?,
            None => refs.head()?,
        };

        return refs.create_ref(name, &start_point);
    }

    Ok(())
}
