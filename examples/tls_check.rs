use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Checking TLS functionality...");
    
    // Check if OpenSSL is available
    match std::process::Command::new("openssl").arg("version").output() {
        Ok(output) => {
            if output.status.success() {
                println!("OpenSSL is available: {}", String::from_utf8_lossy(&output.stdout));
            } else {
                println!("OpenSSL command failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        },
        Err(e) => println!("Failed to run OpenSSL command: {}", e),
    }
    
    // Check for cert and key files
    let cert_exists = std::path::Path::new("cert.pem").exists();
    let key_exists = std::path::Path::new("key.pem").exists();
    
    println!("Certificate file exists: {}", cert_exists);
    println!("Key file exists: {}", key_exists);
    
    Ok(())
}
