pub(crate) fn get_user_preferences(user_id: &str) -> Result<UserPreferences, String> {
    // Logic to retrieve user preferences from the database
    if user_id.is_empty() {
        return Err("User ID cannot be empty".to_string());
    }
    // Placeholder for database retrieval logic
    Ok(UserPreferences::default())
}