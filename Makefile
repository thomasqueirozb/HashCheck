compile:
	cargo build --release

install:
	cp hash_check.toml /etc/
	chmod 600 /etc/hash_check.toml
	cp target/release/hash_check /usr/bin/
	mkdir -p /var/lib/hash_check
	
	cp hash_check.timer /lib/systemd/system/
	cp hash_check.service /lib/systemd/system/
	systemctl enable --now hash_check.timer

uninstall:
	rm /etc/hash_check.toml
	rm /usr/bin/hash_check
	
	systemctl disable hash_check.timer
	rm /lib/systemd/system/hash_check.timer
	rm /lib/systemd/system/hash_check.service

