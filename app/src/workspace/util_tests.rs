use super::chrome_opacity_for;

#[test]
fn chrome_follows_the_configured_opacity_when_blur_is_available() {
    assert_eq!(chrome_opacity_for(85, true), 85);
    assert_eq!(chrome_opacity_for(100, true), 100);
}

#[test]
fn chrome_is_solid_when_the_platform_cannot_blur_behind_it() {
    assert_eq!(chrome_opacity_for(85, false), 100);
    assert_eq!(chrome_opacity_for(1, false), 100);
}
