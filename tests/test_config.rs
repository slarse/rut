use rut::config;
use std::fs;

#[test]
fn test_parse_gitconfig_with_values() {
    // arrange
    let gitconfig_content = "[user]\nname = John Doe\nemail = john@doe.com";
    let tempdir = rut_testhelpers::create_temporary_directory();
    let gitconfig_path = tempdir.join(".gitconfig");
    fs::write(&gitconfig_path, gitconfig_content).unwrap();

    // act
    let parsed_config = config::parse_gitconfig(&gitconfig_path).unwrap();

    // assert
    assert_eq!(parsed_config.name, Some("John Doe".to_string()));
    assert_eq!(parsed_config.email, Some("john@doe.com".to_string()));
}
