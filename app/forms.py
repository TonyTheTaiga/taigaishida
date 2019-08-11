from flask_wtf import FlaskForm
from wtforms import StringField, SubmitField
from wtforms.validators import DataRequired, Email


class ContactForm(FlaskForm):
    Name = StringField("Name", validators=[DataRequired()])

    Email = StringField("Email", validators=[DataRequired(), Email()])

    Phone = StringField("Phone Number", validators=[DataRequired()])

    Message = StringField("Message", validators=[DataRequired()])

    Submit = SubmitField("Send")
