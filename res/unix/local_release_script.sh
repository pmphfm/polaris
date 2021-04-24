set -x

mkdir tmp
cd tmp

if [ -d "polaris-server" ]; then
	cd polaris-server
	git fetch && git rebase origin/master || exit 1
	rm -rf release
	cd ..
else
	git clone git@github.com:pmphfm/polaris.git polaris-server || exit 1
fi
if [ -d "polaris-web" ]; then
	cd polaris-web
	git fetch && git rebase origin/master || exit 1
	rm -rf dist || exit 1
	cd ..
else
       	git clone git@github.com:pmphfm/polaris-web.git || exit 1
fi

if [ -d "docker-polaris" ]; then
	cd docker-polaris
	git fetch && git rebase origin/master || exit 1
	cd ..
else
       	git clone https://github.com/pmphfm/docker-polaris.git docker-polaris || exit 1
fi


cd polaris-server || exit 1
cargo build --release || exit 1
mkdir -p release/tmp/polaris || exit 1


cd ../polaris-web || exit 1
node --version
npm --version
npm ci || exit 1
npm run production || exit 1
cp -R dist ../polaris-server/release/tmp/polaris/web || exit 1


cd ../polaris-server || exit 1

cp -r docs/swagger src migrations test-data build.rs Cargo.toml Cargo.lock rust-toolchain res/unix/Makefile release/tmp/polaris || exit 1
tar -zc -C release/tmp -f ../docker-polaris/polaris.tar.gz polaris || exit 1

cd ../docker-polaris

make docker-build || exit 1
make docker-test || exit 1
make docker-save || exit 1
docker system prune -a -f
