-- Grant Admin Access to Perpngn44
-- Run this SQL script in your PostgreSQL database

-- Update user role to admin by joining with github_accounts
UPDATE users 
SET role = 'admin' 
FROM github_accounts 
WHERE users.id = github_accounts.user_id 
  AND github_accounts.login = 'Perpngn44';

-- Verify the update
SELECT u.id, u.role, u.display_name, g.login, u.created_at 
FROM users u
JOIN github_accounts g ON u.id = g.user_id
WHERE g.login = 'Perpngn44';
