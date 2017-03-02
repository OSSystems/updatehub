package libarchive

// FIXME: test this whole file

/*
#cgo pkg-config: libarchive
#include <archive.h>
#include <archive_entry.h>
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"io"
	"unsafe"
)

// Archive is a wrapper for "C.struct_archive"
type Archive struct {
	archive *C.struct_archive
}

// ArchiveEntry is a wrapper for "C.struct_archive_entry"
type ArchiveEntry struct {
	entry *C.struct_archive_entry
}

// API is the libarchive API provided by this package
type API interface {
	NewRead() Archive
	ReadSupportFilterAll(a Archive)
	ReadSupportFormatRaw(a Archive)
	ReadSupportFormatEmpty(a Archive)
	ReadOpenFileName(a Archive, filename string, blockSize int) error
	ReadFree(a Archive)
	ReadNextHeader(a Archive, e ArchiveEntry) error
	ReadData(a Archive, buffer []byte, length int) (int, error)
}

// LibArchive is the default implementation of API
type LibArchive struct {
}

// NewRead is a wrapper for "C.archive_read_new()"
func (la LibArchive) NewRead() Archive {
	a := Archive{}
	a.archive = C.archive_read_new()
	return a
}

// ReadSupportFilterAll is a wrapper for "C.archive_read_support_filter_all()"
func (la LibArchive) ReadSupportFilterAll(a Archive) {
	C.archive_read_support_filter_all(a.archive)
}

// ReadSupportFormatRaw is a wrapper for "C.archive_read_support_format_raw()"
func (la LibArchive) ReadSupportFormatRaw(a Archive) {
	C.archive_read_support_format_raw(a.archive)
}

// ReadSupportFormatEmpty is a wrapper for "C.archive_read_support_format_empty()"
func (la LibArchive) ReadSupportFormatEmpty(a Archive) {
	C.archive_read_support_format_empty(a.archive)
}

// ReadOpenFileName is a wrapper for "C.archive_read_open_filename()"
func (la LibArchive) ReadOpenFileName(a Archive, filename string, blockSize int) error {
	cFilename := C.CString(filename)
	r := C.archive_read_open_filename(a.archive, cFilename, C.size_t(blockSize))
	C.free(unsafe.Pointer(cFilename))

	if r != C.ARCHIVE_OK {
		return fmt.Errorf(C.GoString(C.archive_error_string(a.archive)))
	}

	return nil
}

// ReadFree is a wrapper for "C.archive_read_free()"
func (la LibArchive) ReadFree(a Archive) {
	C.archive_read_free(a.archive)
}

// ReadNextHeader is a wrapper for "C.archive_read_next_header()"
func (la LibArchive) ReadNextHeader(a Archive, e ArchiveEntry) error {
	r := C.archive_read_next_header(a.archive, &e.entry)

	if r == C.ARCHIVE_EOF {
		return io.EOF
	}

	if r != C.ARCHIVE_OK {
		return fmt.Errorf(C.GoString(C.archive_error_string(a.archive)))
	}

	return nil
}

// ReadData is a wrapper for "C.archive_read_data()"
func (la LibArchive) ReadData(a Archive, buffer []byte, length int) (int, error) {
	r := C.archive_read_data(a.archive, unsafe.Pointer(&buffer[0]), C.size_t(length))

	if r < 0 {
		return int(r), fmt.Errorf(C.GoString(C.archive_error_string(a.archive)))
	}

	return int(r), nil
}

// Reader is an abstraction that implements the io.Reader interface
type Reader struct {
	API               // the implementation being used
	Archive   Archive // the Archive being used
	ChunkSize int     // the chunk size being used
}

// NewReader is a factory method used to create a new Reader. Must
// receive an API implementation, the filePath on which the file will
// be read and the chunkSize used for reading
func NewReader(api API, filePath string, chunkSize int) (*Reader, error) {
	a := api.NewRead()

	api.ReadSupportFilterAll(a)
	api.ReadSupportFormatRaw(a)
	api.ReadSupportFormatEmpty(a)

	err := api.ReadOpenFileName(a, filePath, chunkSize)
	if err != nil {
		api.ReadFree(a)
		return nil, err
	}

	r := &Reader{api, a, chunkSize}

	return r, nil
}

// Read implements the io.Reader interface
func (r Reader) Read(p []byte) (n int, err error) {
	n, err = r.API.ReadData(r.Archive, p, r.ChunkSize)

	if n < 0 && err != nil {
		return 0, err
	}

	if n == 0 && err == nil {
		return 0, io.EOF
	}

	return n, err
}

// ReadNextHeader setups the Archive for a set of reads
func (r Reader) ReadNextHeader() error {
	e := ArchiveEntry{}
	return r.API.ReadNextHeader(r.Archive, e)
}

// Free frees the Archive
func (r Reader) Free() {
	r.API.ReadFree(r.Archive)
}