import unittest, sys

sys.path.append('..')

from app import website


class TestWebsite(unittest.TestCase):
    def setUp(self):
        self.app = website.test_client()
        self.app.testing = True

    def test_homepage_sc(self):
        response = self.app.get('/')
        self.assertEqual(response.status_code, 200)



if __name__ == '__main__':
    unittest.main()
